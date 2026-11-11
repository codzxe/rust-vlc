//! The device-hotplug hooks (card 7.C1; doc 03 §4.9(d) + §4.11(d),
//! doc 05 §3.2 `audio.device-lost`/`-changed`,
//! `video.backend-changed`).
//!
//! # What this module is
//!
//! The **engine-side executor half** of the environment-recovery
//! policies: the typed entry points a host calls when the *world
//! around the media* changes mid-playback. The policy cores live in
//! [`crate::recovery`] (the [`AudioDeviceGuard`]'s bounded
//! suspend/reopen window — card 3.C3); this module is where those
//! policies meet the real [`Engine`]:
//!
//! - [`Engine::observe_audio_device_lost`] — the audio seam reports
//!   the active device went away. The engine emits the wire
//!   `audio.device-lost` fact, then applies the session's `Pause` row
//!   **in-pipeline** (the suspend of doc 03 §4.9(d): freeze the media
//!   clock at the loss position — no teardown, no restart, the
//!   session and its pipeline generation survive untouched).
//! - [`Engine::poll_hotplug`] — drive the reopen window on the
//!   caller's virtual time. Answers
//!   [`HotplugStep::ReopenDevice`] when an attempt is due; emits the
//!   one-time non-fatal `AudioDeviceSuspended` notice when the window
//!   expires (the session stays suspended at the remembered position
//!   — a non-fatal `playback.error`, never a session kill).
//! - [`Engine::observe_audio_reopen`] — the host reports a reopen
//!   attempt's outcome. Success emits `audio.device-changed` and
//!   applies the `Play` row (resume from the *remembered* position:
//!   the clock continues from its own frozen value — the media never
//!   restarts from zero). Failure advances the backoff schedule
//!   inside the window.
//! - [`Engine::observe_renderer_backend`] — the host relays the
//!   renderer's typed capability report (the
//!   renderer-wgpu fallback chain's outcome) into the engine's ONE
//!   wire event stream as `video.backend-changed`.
//!
//! # Event ownership (the one-bridge rule)
//!
//! The guards answer wire-form [`commands::EventKind`]s for hosts
//! that bridge them **directly** (an input layer below the engine's
//! sight — the `recovery.rs` doctrine). A host that drives the
//! hotplug hooks must NOT also bridge the guard's own emissions:
//! the engine owns the emission here, stamped with the session
//! envelope and the engine-global `seq`. The payloads are identical
//! by construction (both derive from the guard's state; the contract
//! suite pins the join).
//!
//! # Ordering (doc 05 §3.3 — the C2 rule)
//!
//! The fact precedes the reaction on the wire: `audio.device-lost`
//! lands before the suspend's `state-changed → paused`;
//! `audio.device-changed` lands before the resume's
//! `state-changed → playing`. The suspend notice
//! (`AudioDeviceSuspended`) is terminal for its episode — nothing
//! follows it until a fresh loss or a late reopen.
//!
//! # Time doctrine
//!
//! Every window takes `now_ns` **from the caller** (the recovery
//! doctrine): the hooks are deterministic under virtual time and
//! never read a clock, never sleep. [`Engine::poll`] deliberately
//! does NOT drive the reopen window — it has no `now_ns` to give;
//! hosts call [`Engine::poll_hotplug`] on their own cadence (the
//! same call shape the platform event queues impose).
//!
//! # What is deliberately NOT here
//!
//! Device *detection*: the OS seams (platform-* `DeviceEventQueue`)
//! and the renderer's own negotiation own observation; the hooks
//! consume typed facts. Auto-resume against the user's explicit
//! pause: a loss that arrives while already paused records the loss
//! but applies nothing (the `Pause` row from `Paused` is a
//! self-transition confirmation — harmless, but the suspend
//! position is whatever the user paused at).
//!
//! # Allocation note (D-1)
//!
//! The hooks run at *event rate* (human-timescale device changes),
//! never per tick: the steady-state cost of carrying this state is
//! the guard's own (zero — see `recovery.rs`). Each hook call
//! allocates only the `Vec` of outputs the apply itself produces,
//! exactly like a submitted command.

use crate::event::EngineEventKind;
use crate::recovery::{AudioDeviceGuard, RecoveryAction, RetryWindow};
use crate::session::{Engine, InputContext, clock_position_ns};
use crate::state::SessionInput;
use media_types::EngineError;

/// What the host should do next, as decided by the hotplug hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugStep {
    /// Nothing due — no host action.
    None,
    /// Attempt to re-acquire the audio output device now (the reopen
    /// seam is the host's — doc 03 §4.9's `AudioBackend::open`; the
    /// engine owns *when*, the host owns *how*). Report the outcome
    /// through [`Engine::observe_audio_reopen`].
    ReopenDevice,
}

impl HotplugStep {
    /// Whether the host owes an action.
    #[must_use]
    pub fn is_action(self) -> bool {
        matches!(self, HotplugStep::ReopenDevice)
    }
}

/// The hotplug observability surface (hosts and tests read this;
/// the wire events are the user-visible half).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HotplugStats {
    /// Device-loss episodes observed (lifetime of the engine).
    pub losses: u64,
    /// Successful re-opens (lifetime).
    pub reopens: u64,
    /// Whether the audio path is currently suspended (device lost,
    /// reopen not yet succeeded).
    pub suspended: bool,
    /// Whether the one-time non-fatal suspension notice has been
    /// emitted for the current episode.
    pub notified: bool,
    /// The remembered suspend position (ns), while suspended.
    pub suspended_position_ns: Option<i64>,
    /// The last renderer backend report relayed through the engine
    /// (backend id + reason), if any.
    pub renderer_backend: Option<(String, String)>,
}

/// The engine-side hotplug state: the audio guard plus the cached
/// renderer backend report.
#[derive(Debug)]
pub(crate) struct HotplugState {
    audio: AudioDeviceGuard,
    /// The last backend report (backend id, reason) — cached so the
    /// *initial* negotiation report (which happens before any
    /// session exists) can be replayed onto the first session's
    /// event stream at open (doc 05 §3.2: "the initial
    /// auto-negotiation report").
    backend_report: Option<(String, String)>,
}

impl Default for HotplugState {
    fn default() -> HotplugState {
        HotplugState {
            audio: AudioDeviceGuard::new(RetryWindow::DEFAULT),
            backend_report: None,
        }
    }
}

impl HotplugState {
    /// The observability snapshot.
    pub(crate) fn stats(&self) -> HotplugStats {
        HotplugStats {
            losses: self.audio.losses(),
            reopens: self.audio.reopens(),
            suspended: self.audio.suspended(),
            notified: self.audio.notified(),
            suspended_position_ns: self.audio.suspended_position_ns(),
            renderer_backend: self.backend_report.clone(),
        }
    }
}

impl Engine {
    /// Observe the loss of the active audio output device at virtual
    /// time `now_ns` (the OS seam's `SinkEvent::DeviceLost` — doc 03
    /// §4.9(d)).
    ///
    /// Emits the wire `audio.device-lost`, then applies the session's
    /// `Pause` row in-pipeline: the media clock freezes at the loss
    /// position (the guard remembers it for the resume), the wire
    /// shows `state-changed → paused`. Nothing is torn down — the
    /// session, its epoch and its pipeline generation all survive
    /// (the no-restart contract: a later reopen resumes mid-media).
    ///
    /// With no live session the loss is still recorded in
    /// [`Engine::hotplug_stats`] (the guard's truth), but nothing is
    /// emitted — there is no session whose stream could carry it.
    pub fn observe_audio_device_lost(&mut self, now_ns: u64, device_id: &str) -> HotplugStep {
        let position_ns = self
            .session
            .as_ref()
            .map(|session| clock_position_ns(&session.runtime.clock))
            .unwrap_or(0);
        let step = self
            .hotplug
            .audio
            .observe_device_lost(now_ns, device_id, position_ns);
        // The fact first (doc 05 §3.3): the loss precedes the
        // suspend reaction on the wire. The guard's own wire-form
        // event is NOT re-bridged — this emission is the one bridge
        // (the module docs' ownership rule).
        self.push_session_event(EngineEventKind::AudioDeviceLost {
            device_id: device_id.to_string(),
        });
        // The suspend: the `Pause` row through the table, exactly as
        // a submitted `playback.pause` would land (the recovery
        // doctrine's command-layer realization — engine-internal
        // applies carry `origin: None`, the implicit-close
        // precedent). A typed rejection (nothing playing to suspend)
        // is consumed honestly: the loss fact and the guard's window
        // stand regardless. The clock freeze is the Pause row's own
        // effect; the position the guard remembered is the resume's
        // anchor.
        if let RecoveryAction::SuspendClock { .. } = step.action {
            self.apply_hotplug_input(SessionInput::Pause);
        }
        // Nothing for the host yet — the first reopen becomes due in
        // poll_hotplug (the guard schedules it at `now_ns`).
        HotplugStep::None
    }

    /// Drive the audio reopen window at virtual time `now_ns`.
    ///
    /// Answers [`HotplugStep::ReopenDevice`] when an attempt is due
    /// (the host performs it and reports back through
    /// [`Engine::observe_audio_reopen`]); emits the one-time
    /// non-fatal suspension notice ([`EngineEventKind::
    /// AudioDeviceSuspended`], wire `playback.error` with
    /// `fatal: false`) when the window expired — the session stays
    /// suspended at the remembered position, alive and closeable.
    pub fn poll_hotplug(&mut self, now_ns: u64) -> HotplugStep {
        let step = self.hotplug.audio.poll(now_ns);
        match step.action {
            RecoveryAction::ReopenDevice => HotplugStep::ReopenDevice,
            RecoveryAction::None => {
                // The window may have just expired: the notice (if
                // the guard emitted one) becomes the engine-side
                // suspension event. Exactly once — the guard's own
                // `notified` latch is the single-shot rule.
                self.relay_suspend_notice(&step);
                HotplugStep::None
            }
            // The audio guard answers only these two actions on poll
            // (its SuspendClock belongs to observe_audio_device_lost
            // and ResumeClock to observe_audio_reopen; FailSource,
            // RetryRead and the surface action are the other
            // guards'). The catch-all is the honest unreachable:
            // consumed, not panicked on (a policy change lands with
            // its tests).
            RecoveryAction::RetryRead { .. }
            | RecoveryAction::FailSource(_)
            | RecoveryAction::ResumeClock { .. }
            | RecoveryAction::SuspendClock { .. }
            | RecoveryAction::RequestSurfaceRecreate => HotplugStep::None,
        }
    }

    /// Report the outcome of a reopen attempt at virtual time
    /// `now_ns`.
    ///
    /// - `Ok(new_device_id)` — the device is re-acquired: emits the
    ///   wire `audio.device-changed`, then applies the session's
    ///   `Play` row (the clock resumes from its own frozen value —
    ///   the remembered position; the media never restarts from
    ///   zero).
    /// - `Err(err)` — the attempt failed: the guard advances its
    ///   backoff schedule inside the window (or emits the one-time
    ///   non-fatal notice when the budget is spent — see
    ///   [`Engine::poll_hotplug`]).
    pub fn observe_audio_reopen(
        &mut self,
        now_ns: u64,
        result: Result<String, EngineError>,
    ) -> HotplugStep {
        let step = self.hotplug.audio.observe_reopen_result(now_ns, result);
        match step.action {
            RecoveryAction::ResumeClock { .. } => {
                // The cause precedes the effect (doc 05 §3.3):
                // device-changed lands before `state-changed →
                // playing`.
                let device_id = self
                    .hotplug
                    .audio
                    .active_device()
                    .unwrap_or_default()
                    .to_string();
                self.push_session_event(EngineEventKind::AudioDeviceChanged { device_id });
                self.apply_hotplug_input(SessionInput::Play);
                HotplugStep::None
            }
            _ => {
                // A failed attempt: the guard advanced its schedule —
                // or spent the budget and answered the one-time
                // notice, which becomes the engine-side suspension
                // event here (the host's next poll_hotplug stays
                // quiet: the guard's latch is the single-shot rule).
                self.relay_suspend_notice(&step);
                HotplugStep::None
            }
        }
    }

    /// Relay the renderer's typed capability report into the engine's
    /// event stream (card 7.C1: "auto-negotiated wgpu backend →
    /// software present; capability report surfaced to UI").
    ///
    /// The renderer owns the negotiation (the renderer-wgpu fallback
    /// chain); the engine owns the ONE wire event stream — this is
    /// the join. `backend` is the now-active backend id (redacted
    /// vocabulary, e.g. `"software"`); `reason` is the negotiation
    /// outcome / fallback cause (display-safe, V8).
    ///
    /// The report is cached: with no live session (the startup
    /// negotiation — before any open) it is replayed onto the first
    /// session's stream at [`Engine::open`], the catalog row's
    /// "initial auto-negotiation report".
    pub fn observe_renderer_backend(&mut self, backend: &str, reason: &str) {
        self.hotplug.backend_report = Some((backend.to_string(), reason.to_string()));
        self.push_session_event(EngineEventKind::RendererBackendReport {
            backend: backend.to_string(),
            reason: reason.to_string(),
        });
    }

    /// The hotplug observability snapshot (losses, reopens, the
    /// suspend state, the cached backend report).
    #[must_use]
    pub fn hotplug_stats(&self) -> HotplugStats {
        self.hotplug.stats()
    }

    /// Apply one input through the table on the live session,
    /// pushing its outputs (engine-internal applies carry
    /// `origin: None` — the implicit-close precedent). A typed
    /// rejection is consumed: the hotplug policies never *require* a
    /// state move (a loss while paused, a resume after a user stop —
    /// the honest answers are no-ops, not errors).
    fn apply_hotplug_input(&mut self, input: SessionInput) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        // The pause-from-Playing immediate tick and every row effect
        // flow exactly as a submitted command's would.
        if let Ok(outputs) = session.apply(input, InputContext::default()) {
            for output in outputs {
                self.outbox.push(output);
            }
        }
    }

    /// Emit one session-stamped engine event onto the outbox, when a
    /// live session's stream can carry it (`None` with no session —
    /// the callers' gate, answered honestly instead of panicking).
    fn push_session_event(&mut self, kind: EngineEventKind) {
        if let Some(output) = self.session.as_ref().map(|session| session.emit_hook(kind)) {
            self.outbox.push(output);
        }
    }

    /// Translate the guard's one-time non-fatal suspension notice
    /// (the wire `playback.error` the guard answers when its reopen
    /// window expires — by time in `poll`, by budget in
    /// `observe_reopen_result`) into the engine-side
    /// [`EngineEventKind::AudioDeviceSuspended`]. The structured
    /// facts come from the guard's own state (the single source of
    /// truth for the payload); the guard's `notified` latch makes the
    /// emission exactly-once whichever path surfaced it.
    fn relay_suspend_notice(&mut self, step: &crate::recovery::RecoveryStep) {
        if let Some(event) = step.events.first()
            && matches!(
                event,
                commands::EventKind::PlaybackError {
                    stage: commands::Stage::AudioOutput,
                    code: commands::ErrorCode::Device,
                    fatal: false,
                    ..
                }
            )
        {
            let kind = EngineEventKind::AudioDeviceSuspended {
                device_id: self
                    .hotplug
                    .audio
                    .suspended_device()
                    .unwrap_or_default()
                    .to_string(),
                reopen_attempts: self.hotplug.audio.reopen_attempts_used(),
                last_error: self.hotplug.audio.last_reopen_failure(),
            };
            self.push_session_event(kind);
        }
    }

    /// The initial backend report replay at open (the module docs):
    /// called by [`Engine::open`] after the session's announcements.
    pub(crate) fn replay_backend_report_at_open(&mut self) {
        if let Some((backend, reason)) = self.hotplug.backend_report.clone() {
            self.push_session_event(EngineEventKind::RendererBackendReport { backend, reason });
        }
    }
}
