//! The app-layer shell runtime (issue #3 — the 4.H1 app-layer wiring):
//! the exact composition the ADR-004 tauri wrapper drives. Everything
//! the webview host needs, proven headlessly:
//!
//! - [`ShellRuntime::dispatch`] — one UI-originated command: allocate
//!   the request id (§4.3 echo legality), route through THE single
//!   [`lumen_dispatch`](crate::dispatch::lumen_dispatch), observe the
//!   response (§4.2/§4.3 bookkeeping), then drain every new engine
//!   event through the staleness filter into the bounded egress queue.
//! - [`ShellRuntime::pump_once`] + [`PACER_TICK`] — the session pacer
//!   (card 3.B3): one FULL engine pump in the CLI host's own
//!   semantics (poll + time advance + end-tail play-out) followed by
//!   the event drain. The engine is synchronous single-writer — it
//!   advances ONLY when a host pumps it — and between invokes
//!   nothing pumps, so a wall-clock thread calling `pump_once` every
//!   [`PACER_TICK`] (one render quantum, ≈30.6 ms) is what makes GUI
//!   playback progress at all: time-ticks flow, the position
//!   advances, EOS arrives (without it the shell could open/pause/
//!   seek but everything froze at 0 between interactions).
//! - [`ShellRuntime::open_path`] / [`on_files_dropped`] /
//!   [`pick_and_open`] — the file-open acquisitions (dialog,
//!   drag-drop, direct path) funneling through the SAME command —
//!   there is no second file-open mechanism.
//! - [`ShellRuntime::take_pending_events`] — the emitter half: what
//!   the wrapper emits on `"lumen://event"`, in engine order.
//!
//! What this build cannot prove (the honest deferral): the real
//! `tauri::command` wrapper, the `app_handle.emit` calls, and the
//! window build itself — webkit2gtk-4.1 is absent from this sandbox
//! (ADR-004; `apps/desktop/tauri/INFRA-BLOCKED.md`). The runtime is
//! the seam those calls delegate to; every method here runs under
//! `cargo test` today.

use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use commands::envelope::RequestId;
use commands::{Command, CommandOutcome, CommandResult, WireEvent, WireRequest, WireResponse};

use crate::dispatch::BoundedEventSink;
use crate::dispatch::EventQueueStats;
use crate::dispatch::EventSink;
use crate::dispatch::ShellHost;
use crate::dispatch::lumen_dispatch;
use crate::dispatch::open_path_request;
use crate::error::HostError;
use crate::files::DragDropHandler;
use crate::files::NativeFileDialog;
use crate::host::DesktopHost;
use crate::host::EventDelivery;

/// The default egress capacity (the mailbox default's reasoning: one
/// event per command class plus headroom for a burst of time-ticks;
/// overflow drops the OLDEST event, counted — order is never violated).
pub const DEFAULT_EVENT_CAPACITY: NonZeroUsize = {
    match NonZeroUsize::new(64) {
        Some(value) => value,
        // The impossible arm is a compile-time dead end (the 2.C4
        // convention), not a banned-macro escape hatch.
        #[allow(clippy::empty_loop)] // clippy's suggested `panic!()` is workspace-denied
        None => loop {},
    }
};

/// The pacer cadence (card 3.B3): one null-device render quantum —
/// [`lumen_cli::pipeline_host::RENDER_QUANTUM`] frames at the 48 kHz
/// nominal rate = 30 625 µs. This is the same constant the CLI host
/// renders per `advance` tick (the quantum the sample-counter media
/// clock advances by), so a wall-clock thread calling
/// [`ShellRuntime::pump_once`] on this cadence slices wall time into
/// engine ticks at the device's own period — the null-device
/// stand-in for the real audio callback's clock (ADR-005's real
/// backends will own this cadence from the device side). A session
/// at a different rate (44.1 kHz renders 33.33 ms quanta) stays
/// correct at this cadence: the pump renders whatever the ring
/// holds, and the cadence only sets how coarsely wall time is cut,
/// never a second clock.
pub const PACER_TICK: Duration =
    Duration::from_micros((lumen_cli::pipeline_host::RENDER_QUANTUM as u64) * 1_000_000 / 48_000);

/// What one dialog attempt concluded.
#[derive(Debug, Clone)]
pub enum DialogOutcome {
    /// A path was picked and its open command dispatched; the wire
    /// response is the engine's answer (media: `session.open`;
    /// playlist: `playlist.load`). Boxed — the wire response is the
    /// enum's largest field by far and the outcome is moved, not
    /// stored in bulk (clippy's own advice).
    Opened(Box<WireResponse>),
    /// The user canceled the modal — a no-op, never an error.
    Canceled,
    /// The webview layer is not linked in this build (the typed
    /// ADR-004 pending state).
    Pending(HostError),
}

/// The composed shell: correlation (host.rs) + engine routing
/// (dispatch.rs) + the bounded event egress, in one drivable object.
pub struct ShellRuntime {
    host: DesktopHost,
    shell: ShellHost,
    events: BoundedEventSink,
    /// The webview frame tap (w31.1, lane `luf1-channel`): installed
    /// on the first `session.set-frame-channel` bind and reused for
    /// every later bind (the mailbox drains between; one tap, one
    /// bridge, one decoder — the D-1/D-8 discipline).
    frame_tap: Option<std::sync::Arc<crate::frame_tap::FrameTap>>,
    /// The bound channel id (tauri's `__CHANNEL__:<id>` reference,
    /// the bare number on the wire) — taken ONCE by the wrapper's
    /// ticker to build its `tauri::ipc::Channel`, then None (a
    /// re-bind — e.g. a reloaded webview — lands a new id here).
    frame_channel: Option<u32>,
}

impl Default for ShellRuntime {
    fn default() -> Self {
        Self::new(DEFAULT_EVENT_CAPACITY)
    }
}

impl ShellRuntime {
    /// A fresh runtime with an explicit egress capacity.
    #[must_use = "pure constructor"]
    pub fn new(event_capacity: NonZeroUsize) -> ShellRuntime {
        ShellRuntime {
            host: DesktopHost::new(),
            shell: ShellHost::new(),
            events: BoundedEventSink::with_capacity(event_capacity),
            frame_tap: None,
            frame_channel: None,
        }
    }

    /// The engine-driving host (test/diagnostic access; every command
    /// still flows through [`Self::dispatch`]).
    #[must_use = "the adopted composition pattern"]
    pub fn engine_host(&mut self) -> &mut lumen_cli::host::CliHost {
        self.shell.engine_host()
    }

    /// Allocate the next request id (monotonic from 1; the §4.3 log
    /// learns it, so responses to it are legal).
    pub fn next_request_id(&mut self) -> RequestId {
        self.host.next_request_id()
    }

    /// THE dispatch path for UI-originated commands: route through the
    /// single `lumen_dispatch`, observe the response, then drain every
    /// new engine event (§4.2 filter first) into the bounded queue —
    /// the wrapper's per-invoke body.
    pub fn dispatch(&mut self, request: &WireRequest) -> WireResponse {
        // The shell-owned interception (w31.1): the frame-channel bind
        // is SHELL state, never engine state — the CLI host would
        // refuse it unsupported-for-host (correctly: the pure CLI has
        // no webview), so the runtime answers it HERE, before the
        // delegation, installing/refreshing the presented-frame tap.
        // Every other command flows to the engine untouched.
        if let Command::SessionSetFrameChannel { channel } = &request.command {
            return self.bind_frame_channel(request.request_id, *channel);
        }
        let response = lumen_dispatch(&mut self.shell, request);
        let _ = self.host.observe_response(&response);
        self.pump_events();
        response
    }

    /// The `session.set-frame-channel` answer (the intercepted half;
    /// see [`Self::dispatch`]): install the tap (first bind constructs
    /// it — `CliHost::set_present_tap` is state-safe, so a bind BEFORE
    /// the first open stashes and a mid-session bind installs live),
    /// record the channel id for the wrapper's ticker, answer the
    /// class-S `Unit` completion.
    fn bind_frame_channel(&mut self, request_id: RequestId, channel: u32) -> WireResponse {
        let tap = match &self.frame_tap {
            Some(tap) => std::sync::Arc::clone(tap),
            None => {
                let tap = std::sync::Arc::new(
                    crate::frame_tap::FrameTap::new()
                        .unwrap_or_else(|err| unreachable!("the default config is valid: {err}")),
                );
                self.frame_tap = Some(std::sync::Arc::clone(&tap));
                tap
            }
        };
        self.shell.engine_host().set_present_tap(tap);
        self.frame_channel = Some(channel);
        WireResponse {
            request_id,
            session: None,
            generation: commands::envelope::Generation(0),
            result: Ok(CommandOutcome::Completed(CommandResult::Unit)),
        }
    }

    /// The bound channel id, if one has not been taken yet (the
    /// wrapper's ticker calls this once per bind to build its
    /// `tauri::ipc::Channel` via `JavaScriptChannelId`; a None answer
    /// means "nothing new to build").
    #[must_use = "the wrapper's one-shot bind handshake"]
    pub fn take_frame_channel_binding(&mut self) -> Option<u32> {
        self.frame_channel.take()
    }

    /// Whether a frame channel is bound (the ticker skips the drain
    /// when not — no mailbox work, no allocations, an unbound shell
    /// is exactly the pre-w31.1 shell).
    #[must_use = "the ticker's cheap guard"]
    pub fn frame_channel_bound(&self) -> bool {
        self.frame_channel.is_some() || self.frame_tap.is_some()
    }

    /// Drain the frame tap's mailbox into `out` as complete LUF1
    /// records (oldest first; the ticker ships the LAST and drops the
    /// rest — one frame per tick keeps the webview at the pacer's own
    /// cadence). Returns how many records were appended (0 when no
    /// tap exists — the unbound shell's steady state).
    pub fn take_frame_records(&mut self, out: &mut Vec<Vec<u8>>) -> usize {
        match &self.frame_tap {
            Some(tap) => tap.drain_records(out),
            None => 0,
        }
    }

    /// Drain new engine events through the §4.2 staleness filter into
    /// the bounded egress queue (also what a periodic wrapper tick
    /// calls between invokes).
    pub fn pump_events(&mut self) {
        let host = &mut self.host;
        let events = &mut self.events;
        let mut deliver = DeliverThrough { host, events };
        let _ = self.shell.forward_new_events(&mut deliver);
    }

    /// One pacer tick (card 3.B3): a FULL engine pump in the CLI
    /// host's own [`lumen_cli::host::CliHost::pump`] semantics —
    /// `engine.poll()` (the single writer's whole scheduler), the
    /// output drain, the null-device time advance (each rendered
    /// quantum moves the sample counter, the media clock's truth; a
    /// frozen/paused clock pulls nothing, so this is pause-safe
    /// unconditionally), and the end-tail play-out — then the §4.2
    /// event drain into the bounded egress queue. Delegated, not
    /// duplicated: every engine rule lives in the adopted host
    /// (ARCH-5); this is only the cadence seam the wrapper's ticker
    /// drives every [`PACER_TICK`]. Between invokes nothing else
    /// advances the engine, so without this cadence the GUI's playback
    /// freezes between interactions. Safe in every state (idle,
    /// paused, ended): the pump's halves no-op when there is nothing
    /// to advance.
    ///
    /// The whole-ring drain per call is the headless-proof semantics
    /// (the pacer test drives a session to `Ended` through it); the
    /// production ticker drives [`ShellRuntime::pump_session`]
    /// instead — the wall-bounded advance below — which renders only
    /// what the wall is owed and cannot burst-play a full ring on a
    /// slow tick.
    pub fn pump_once(&mut self) {
        self.shell.engine_host().pump();
        self.pump_events();
    }

    /// One **real-time session tick** (the GUI's wall-clock pacer —
    /// the production cadence seam, w27): poll + a **wall-bounded**
    /// time advance ([`CliHost::pump_wallclock`]: the device renders
    /// only the quanta the wall has become owed since the Playing
    /// anchor — never the whole ring) + the end-tail play-out, then
    /// the §4.2 event drain. The engine-side decode park
    /// (`FrameConsumer::ready_for`, card 3.B3's ring→decode
    /// backpressure) keeps the ring bounded between ticks, so this
    /// cadence is stable under any tick jitter: a slow tick catches
    /// up only to the wall, never past it.
    ///
    /// The wrapper calls this on a short ticker under the SAME lock
    /// as dispatches — the engine's single-writer invariant holds
    /// (state moves only inside `Engine::poll`, and `poll` runs only
    /// under the lock). Safe with no session, paused, or buffering:
    /// the pump's own anchor logic no-ops.
    pub fn pump_session(&mut self) {
        self.shell.engine_host().pump_wallclock();
        self.pump_events();
    }

    /// The events awaiting webview emission, in engine order (each
    /// becomes one `lumen://event` emit; the queue empties).
    pub fn take_pending_events(&mut self) -> Vec<WireEvent> {
        self.events.take()
    }

    /// The egress accounting (bounded-queue honesty).
    #[must_use = "observable evidence"]
    pub fn event_stats(&self) -> &EventQueueStats {
        self.events.stats()
    }

    /// Open one shell-acquired path (the direct funnel: dialog pick,
    /// drop translation, or any host integration). Media files open a
    /// session; playlist files load the list; anything else is the
    /// honest no-op (`None`).
    pub fn open_path(&mut self, path: &Path) -> Option<WireResponse> {
        let request_id = self.host.next_request_id();
        let request = open_path_request(request_id, path)?;
        Some(self.dispatch(&request))
    }

    /// Translate one drop event's paths through the handler and
    /// dispatch the result (a drop with no openable file is a no-op).
    pub fn on_files_dropped(
        &mut self,
        handler: &mut dyn DragDropHandler,
        paths: &[PathBuf],
    ) -> Option<WireResponse> {
        let request_id = self.host.next_request_id();
        match handler.on_files_dropped(paths, request_id) {
            Ok(Some(request)) => Some(self.dispatch(&request)),
            Ok(None) => None,
            Err(error) => {
                // The seam's own typed failure: surfaced as the wire
                // error envelope's shape demands a response — the
                // honest refusal, echoed for the allocated id.
                Some(refusal_response(request_id, &error))
            }
        }
    }

    /// Run the native file dialog and dispatch the pick (the wrapper's
    /// dialog-button body). Cancel is a no-op; the not-linked build
    /// answers the typed pending state.
    pub fn pick_and_open(&mut self, dialog: &mut dyn NativeFileDialog) -> DialogOutcome {
        match dialog.pick_media_file() {
            Ok(Some(path)) => match self.open_path(&path) {
                Some(response) => DialogOutcome::Opened(Box::new(response)),
                // The dialog's filter owns pickability; a picked path
                // that classifies to nothing is the honest no-op.
                None => DialogOutcome::Canceled,
            },
            Ok(None) => DialogOutcome::Canceled,
            Err(error) => DialogOutcome::Pending(error),
        }
    }
}

/// The §4.2-filtered delivery: stale events are counted and refused,
/// current events enter the bounded queue (composition, not a copy).
struct DeliverThrough<'a> {
    host: &'a mut DesktopHost,
    events: &'a mut BoundedEventSink,
}

impl EventSink for DeliverThrough<'_> {
    fn forward(&mut self, event: &WireEvent) {
        match self.host.deliver_event(event) {
            EventDelivery::Kept => self.events.forward(event),
            EventDelivery::DroppedStale => self.events.note_stale(),
        }
    }
}

/// The seam-failure answer: a typed error envelope shaped as the wire
/// response the UI already knows how to render (the dialog/drop seams
/// fail host-side, never engine-side — the UI still gets a wire
/// answer, never silence).
fn refusal_response(request_id: RequestId, error: &HostError) -> WireResponse {
    WireResponse {
        request_id,
        session: None,
        generation: commands::envelope::Generation(0),
        result: Err(commands::CommandError::new(
            commands::Stage::Control,
            commands::ErrorCode::NotReady,
            error.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::ErrorCode;
    use commands::EventKind;
    use commands::PlaybackState;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::time::Instant;
    use testkit::Must;

    /// The proof loop's pump bound: 2× the CLI host's own run-to-end
    /// bound (`RUN_PUMPS` = 512 full pumps); the measured run for
    /// THIS fixture is ~170 full pumps (demux-paced — the AVI lane
    /// yields roughly one packet per poll, half of them video), so
    /// the bound is pure headroom, not new arithmetic.
    const PROOF_PUMP_BOUND: usize = 1024;

    /// A scripted dialog — the test half of the acquisition seam.
    struct ScriptedDialog {
        answer: Result<Option<PathBuf>, HostError>,
    }

    impl NativeFileDialog for ScriptedDialog {
        fn pick_media_file(&mut self) -> Result<Option<PathBuf>, HostError> {
            self.answer.clone()
        }
    }

    fn request(id: RequestId, command: Command) -> WireRequest {
        WireRequest {
            request_id: id,
            command,
        }
    }

    /// The full dispatch loop over the REAL engine: a served command
    /// (the settings seam — no session needed) answers completed and
    /// the §4.3 log accepts it because the runtime allocated the id.
    #[test]
    fn dispatch_routes_observes_and_emits() {
        let mut runtime = ShellRuntime::default();
        let id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            id,
            Command::ConfigSetSetting {
                area: "interface".to_string(),
                key: "theme".to_string(),
                value: commands::catalog::types::SettingValue::String("dark".to_string()),
            },
        ));
        assert_eq!(response.request_id, id);
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::SettingApplied {
                applied,
                ..
            })) => {
                assert_eq!(
                    applied,
                    commands::catalog::types::SettingValue::String("dark".to_string())
                );
            }
            other => unreachable!("config.set-setting must complete: {other:?}"),
        }
        // The runtime's egress queue conserves: nothing was offered by
        // a class-S settings command.
        assert!(runtime.event_stats().conserves());
    }

    /// The config round-trip over the single command (issue #3,
    /// handoff d): set → get answers the applied value through the
    /// same dispatch path the UI uses.
    #[test]
    fn config_writes_round_trip_through_dispatch() {
        let mut runtime = ShellRuntime::default();
        let set_id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            set_id,
            Command::ConfigSetSetting {
                area: "interface".to_string(),
                key: "theme".to_string(),
                value: commands::catalog::types::SettingValue::String("dark".to_string()),
            },
        ));
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::SettingApplied {
                applied,
                ..
            })) => assert_eq!(
                applied,
                commands::catalog::types::SettingValue::String("dark".to_string())
            ),
            other => unreachable!("set must complete: {other:?}"),
        }
        let get_id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            get_id,
            Command::ConfigGetSetting {
                area: "interface".to_string(),
                key: "theme".to_string(),
            },
        ));
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::SettingResult {
                value, ..
            })) => assert_eq!(
                value,
                commands::catalog::types::SettingValue::String("dark".to_string())
            ),
            other => unreachable!("get must complete: {other:?}"),
        }
    }

    /// An invalid settings write refuses typed (the engine validates
    /// against the embedded schema — never a silent coercion).
    #[test]
    fn invalid_config_writes_refuse_typed() {
        let mut runtime = ShellRuntime::default();
        let id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            id,
            Command::ConfigSetSetting {
                area: "no-such-area".to_string(),
                key: "no-such-key".to_string(),
                value: commands::catalog::types::SettingValue::Bool(true),
            },
        ));
        match response.result {
            Err(error) => assert_eq!(error.code, ErrorCode::InvalidArgument, "{error}"),
            Ok(_) => unreachable!("an unknown settings key must refuse"),
        }
    }

    /// A real open → settle → events flow reaches the bounded egress
    /// queue in engine order (the live feed the wrapper emits).
    #[test]
    fn engine_events_reach_the_bounded_queue_in_order() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"
        );
        let mut runtime = ShellRuntime::default();
        let response = match runtime.open_path(&PathBuf::from(fixture)) {
            Some(response) => response,
            None => unreachable!("a media file must classify"),
        };
        let _ = response;
        runtime.engine_host().settle();
        runtime.pump_events();
        let events = runtime.take_pending_events();
        assert!(!events.is_empty(), "a settled open must produce events");
        let seqs: Vec<u64> = events.iter().map(|event| event.seq).collect();
        let mut sorted = seqs.clone();
        sorted.sort_unstable();
        assert_eq!(seqs, sorted, "engine order preserved end to end");
        // Monotonic drain: nothing re-delivered.
        assert!(runtime.take_pending_events().is_empty());
        assert!(runtime.event_stats().conserves());
        assert_eq!(runtime.event_stats().taken, events.len() as u64);
    }

    /// A playlist file loads the list, not a session (4.D1's handoff):
    /// the request reaches the engine through the same single command
    /// and answers the honest typed refusal for the unserved id.
    #[test]
    fn playlist_files_load_the_list_through_the_command() {
        let mut runtime = ShellRuntime::default();
        // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        let response = match runtime.open_path(&PathBuf::from("/tmp/queue.m3u")) {
            Some(response) => response,
            None => unreachable!("a playlist file must classify"),
        };
        match response.result {
            Err(error) => {
                assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}");
                assert_eq!(
                    error.details.get("command").map(String::as_str),
                    Some("playlist.load")
                );
            }
            Ok(_) => unreachable!("playlist.load is not served by this engine build"),
        }
        assert_eq!(runtime.engine_host().session_id(), None);
    }

    /// THE card 3.B3 proof — the GUI plays (the wave's key
    /// regression): a real fixture opened through the GUI's own
    /// funnel (`open_path` → the single `lumen_dispatch`), playback
    /// started through the same single command (the play button's
    /// exact path), then ONLY `pump_once` ticks — the pacer thread's
    /// exact body — drive the session to its end. No `settle`, no
    /// host verb, no sleeps: deterministic (every observable is a
    /// pure function of the pump count), bounded by iteration count
    /// and a wall-clock deadline (the deadline is the hang guard,
    /// never a pacing input). Before the pacer this was the
    /// frozen-GUI failure: between invokes nothing pumped, so the
    /// position never advanced, time-ticks never flowed, and EOS
    /// never arrived.
    #[test]
    fn pacer_proof_pump_once_drives_a_real_session_to_end() {
        // The same 2.000 s fixture the CLI's conformance lane runs to
        // its end with the `advance` verb (both limbs decodable — the
        // proof covers the whole pipeline, not one lane).
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"
        );
        let mut runtime = ShellRuntime::default();
        let open = match runtime.open_path(&PathBuf::from(fixture)) {
            Some(response) => response,
            None => unreachable!("a media file must classify"),
        };
        match open.result {
            Ok(CommandOutcome::Accepted { .. }) => {}
            other => unreachable!("a real media file must open: {other:?}"),
        }
        // Start playback through the same dispatch path (C8-safe from
        // every live state: the open may still be settling).
        let play_id = runtime.next_request_id();
        let play = runtime.dispatch(&request(play_id, Command::PlaybackPlay));
        match play.result {
            Ok(CommandOutcome::Accepted { .. }) => {}
            other => unreachable!("play must be accepted: {other:?}"),
        }
        // The pacer loop — the ticker's exact body, driven to the
        // terminal state. The first take also drains the open's own
        // event burst (the queue the dispatch path filled).
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut saw_playing = false;
        let mut time_ticks = 0usize;
        let mut saw_end_of_media = false;
        let mut ended = false;
        let mut pumps = 0usize;
        while pumps < PROOF_PUMP_BOUND {
            pumps += 1;
            runtime.pump_once();
            for event in runtime.take_pending_events() {
                match event.kind {
                    EventKind::SessionStateChanged { to, .. } => match to {
                        PlaybackState::Playing => saw_playing = true,
                        PlaybackState::Ended => ended = true,
                        _ => {}
                    },
                    EventKind::TimingTimeTick { .. } => time_ticks += 1,
                    EventKind::PlaybackEndOfMedia => saw_end_of_media = true,
                    _ => {}
                }
            }
            if ended {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "the pacer loop must not hang: a bound breach is a failure, never a mystery"
            );
        }
        assert!(saw_playing, "the pacer must reach Playing ({pumps} pumps)");
        assert!(
            time_ticks > 0,
            "the pacer must flow time-ticks ({pumps} pumps)"
        );
        assert!(
            saw_end_of_media,
            "EOS must arrive through the pacer ({pumps} pumps)"
        );
        assert!(
            ended,
            "the pacer must reach Ended within {PROOF_PUMP_BOUND} pumps \
             ({pumps} pumped, {time_ticks} time-ticks)"
        );
        // The egress accounting conserves end to end.
        assert!(runtime.event_stats().conserves());
    }

    /// The frame-channel bind intercepts before the engine (w31.1):
    /// the shell-owned command answers the class-S `Unit` completion
    /// itself, the channel id lands in the one-shot bind handshake,
    /// and the engine host NEVER sees the command (the CLI would
    /// refuse it unsupported-for-host — correctly: it has no webview).
    #[test]
    fn frame_channel_bind_is_intercepted_by_the_shell() {
        let mut runtime = ShellRuntime::default();
        let id = runtime.next_request_id();
        let response =
            runtime.dispatch(&request(id, Command::SessionSetFrameChannel { channel: 7 }));
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::Unit)) => {}
            other => unreachable!("the bind must complete Unit: {other:?}"),
        }
        assert_eq!(runtime.take_frame_channel_binding(), Some(7));
        assert_eq!(runtime.take_frame_channel_binding(), None, "one-shot");
        assert!(runtime.frame_channel_bound());
    }

    /// THE w31.1 proof — the GUI's own funnel, end to end with frames:
    /// bind the channel FIRST (the real UI's startup order: the
    /// webview registers its consumer before opening anything), open
    /// the fixture, play, then ONLY `pump_once` ticks drive the
    /// session — and LUF1 frame records flow out of the same pacer
    /// the w30.7 proof drives. Before w31.1 the GUI decoded and
    /// dropped; this is the regression bar for "the GUI shows video".
    #[test]
    fn frame_channel_proof_pump_once_ships_luf1_records() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"
        );
        let mut runtime = ShellRuntime::default();
        // Bind first: the pre-open stash rides the first consumer.
        let bind_id = runtime.next_request_id();
        let bind = runtime.dispatch(&request(
            bind_id,
            Command::SessionSetFrameChannel { channel: 1 },
        ));
        assert!(matches!(
            bind.result,
            Ok(CommandOutcome::Completed(commands::CommandResult::Unit))
        ));
        assert_eq!(runtime.take_frame_channel_binding(), Some(1));

        let open = match runtime.open_path(&std::path::PathBuf::from(fixture)) {
            Some(response) => response,
            None => unreachable!("a media file must classify"),
        };
        match open.result {
            Ok(CommandOutcome::Accepted { .. }) => {}
            other => unreachable!("a real media file must open: {other:?}"),
        }
        let play_id = runtime.next_request_id();
        let play = runtime.dispatch(&request(play_id, Command::PlaybackPlay));
        assert!(
            matches!(play.result, Ok(CommandOutcome::Accepted { .. })),
            "play must be accepted: {:?}",
            play.result
        );

        // The pacer loop — the ticker's exact body, plus the frame
        // drain the ticker performs after each pump.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let mut frames = Vec::new();
        let mut records = 0usize;
        let mut ended = false;
        for _ in 0..PROOF_PUMP_BOUND {
            runtime.pump_once();
            for event in runtime.take_pending_events() {
                if matches!(event.kind, commands::EventKind::PlaybackEndOfMedia) {
                    ended = true;
                }
            }
            records += runtime.take_frame_records(&mut frames);
            if ended {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the frame pacer loop must not hang"
            );
        }
        assert!(ended, "the session must end ({records} frame records)");
        assert!(
            records > 0,
            "LUF1 frame records must flow through the pacer (the GUI-shows-video bar)"
        );
        assert!(
            frames.iter().all(|record| record.starts_with(b"LUF1")),
            "every record carries the LUF1 magic"
        );
        // The egress accounting conserves end to end.
        assert!(runtime.event_stats().conserves());
    }

    /// The pacer cadence is derived, not invented: one render quantum
    /// of the null device — the CLI host's own `RENDER_QUANTUM`
    /// (1470 frames) at the 48 kHz nominal rate. Pinned so a change
    /// on either side is a visible diff here, not a silent rate drift.
    #[test]
    fn pacer_tick_is_one_render_quantum() {
        assert_eq!(PACER_TICK, Duration::from_micros(30_625));
        assert_eq!(
            PACER_TICK,
            Duration::from_micros(
                (lumen_cli::pipeline_host::RENDER_QUANTUM as u64) * 1_000_000 / 48_000
            )
        );
    }

    /// Non-openable paths are honest no-ops (never a session error the
    /// UI must handle).
    #[test]
    fn non_openable_paths_are_noops() {
        let mut runtime = ShellRuntime::default();
        assert!(
            runtime
                .open_path(&PathBuf::from("/tmp/README.md")) // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
                .is_none()
        );
        assert!(
            runtime
                .open_path(&PathBuf::from("/tmp/queue.unknown-ext")) // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
                .is_none()
        );
    }

    /// The drop funnel through the REAL handler: first openable file
    /// wins, playlist classification included.
    #[test]
    fn drops_funnel_through_the_runtime() {
        let mut runtime = ShellRuntime::default();
        let mut dnd = crate::files::PathDragDrop;
        let response = match runtime.on_files_dropped(
            &mut dnd,
            &[
                PathBuf::from("/tmp/notes.txt"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
                PathBuf::from("/tmp/movie.avi"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ],
        ) {
            Some(response) => response,
            None => unreachable!("a media file was dropped"),
        };
        // The open path's own honest answer decides (the fixture is
        // real); what is proven here is the FUNNEL, not the decode.
        let _ = response;
        // A drop with nothing openable is a no-op.
        assert!(
            runtime
                .on_files_dropped(&mut dnd, &[PathBuf::from("/tmp/README.md")]) // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
                .is_none()
        );
    }

    /// The dialog wiring: a pick dispatches through the same command;
    /// cancel is a no-op; the not-linked build answers the typed
    /// pending state.
    #[test]
    fn dialog_picks_dispatch_cancels_noop_and_pending() {
        let mut runtime = ShellRuntime::default();
        // Canceled: a no-op, never an error.
        let mut canceled = ScriptedDialog { answer: Ok(None) };
        assert!(matches!(
            runtime.pick_and_open(&mut canceled),
            DialogOutcome::Canceled
        ));
        // Pending: the typed ADR-004 state, verbatim.
        let mut pending = ScriptedDialog {
            answer: Err(HostError::WebviewPending { adr: "ADR-004" }),
        };
        match runtime.pick_and_open(&mut pending) {
            DialogOutcome::Pending(error) => {
                assert_eq!(error, HostError::WebviewPending { adr: "ADR-004" });
            }
            other => unreachable!("no webview is linked: {other:?}"),
        }
        // Picked: the file-open funnel (an unserved path class still
        // answers through the command — here a playlist, refused typed).
        let mut picked = ScriptedDialog {
            answer: Ok(Some(PathBuf::from("/tmp/queue.pls"))), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        };
        match runtime.pick_and_open(&mut picked) {
            DialogOutcome::Opened(response) => match response.result {
                Err(error) => assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}"),
                Ok(_) => unreachable!("playlist.load is not served by this engine build"),
            },
            other => unreachable!("a playlist file was picked: {other:?}"),
        }
    }

    /// The bounded egress queue: overflow drops the OLDEST event and
    /// the stats conserve (the mailbox discipline, applied to events).
    #[test]
    fn egress_queue_is_bounded_drop_oldest_and_conserves() {
        let capacity = NonZeroUsize::new(2).must();
        let mut sink = BoundedEventSink::with_capacity(capacity);
        // Drive the queue directly through the EventSink trait (the
        // runtime path is covered above; this pins the bound itself).
        for seq in 0..5u64 {
            sink.forward(&WireEvent {
                seq,
                session: None,
                generation: None,
                origin: None,
                kind: commands::EventKind::PlaybackEndOfMedia,
            });
        }
        let drained = sink.take();
        assert_eq!(
            drained.iter().map(|event| event.seq).collect::<Vec<_>>(),
            vec![3, 4],
            "the oldest events dropped, order preserved"
        );
        let stats = sink.stats();
        assert_eq!(stats.offered, 5);
        assert_eq!(stats.dropped_overflow, 3);
        assert!(stats.conserves());
        assert_eq!(stats.dropped_stale, 0);
    }
}
