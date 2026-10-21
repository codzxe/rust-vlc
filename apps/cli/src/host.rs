//! The CLI host (card 2.D2): owns the `playback-engine` [`Engine`],
//! pumps it, correlates receipts to requests, records the event
//! stream, and answers the class-S snapshot commands.
//!
//! # The host IS the session thread (2.C2's invariant S1)
//!
//! The minimal engine is synchronous and single-writer: state moves
//! only inside [`Engine::poll`]. The CLI's [`CliHost::pump`] is
//! therefore the whole scheduler — one pump is
//!
//! 1. `engine.poll()` — drain the command inbox (transition table),
//!    then one bounded pipeline tick (demux ≤ 64 events, decode ≤ 16
//!    packets per limb),
//! 2. drain the output hub — receipts first (they precede the events
//!    they caused, doc 05 §4.3), then events through the wire
//!    conversion and the staleness filter,
//! 3. **advance time**: render audio quanta from the null device's
//!    ring (each render moves the sample counter → the audio-master
//!    clock), ticking the video scheduler once per quantum so
//!    presents track the clock.
//!
//! Deterministic by construction on the audio-bearing path: nothing
//! reads a wall clock, and every observable (states, events, frames
//! staged, samples rendered) is a pure function of the pump count.
//!
//! # Seek (the card's honest composition)
//!
//! The engine's seek input is card 3.A1. The CLI composes
//! `playback.seek` from the primitives that DO exist — every step a
//! real engine transition with real generation minting:
//!
//! 1. resolve the target (absolute µs; relative against the live
//!    clock position; percent against the demuxed duration), clamp to
//!    `[0, duration]` (doc 05 §2.4 — clamping is truth),
//! 2. `playback.stop` from `Playing`/`Paused` (gen-bumping teardown),
//! 3. `session.close` (session destroyed → `Idle`),
//! 4. `session.open` at the target: the demuxer's real seek query
//!    validates the landing, the skip wrapper drops pre-target
//!    packets, the clock anchors at the target.
//!
//! `[PROPOSAL]` — the session id changes across a seek (a reopen),
//! and the state trail shows `Paused/Playing → Stopped → Idle →
//! Opening → Buffering → Playing`. That is the honest 2.D2 shape;
//! in-session seeks with generation-carrying pipelines are 3.A1.

use std::collections::BTreeMap;
use std::sync::Arc;

use audio_output::NullDevice;
use commands::catalog::command::{CommandListing, CommandResult};
use commands::catalog::types::{
    Chapter, DecoderInfo, Duration, EngineInfo, MediaInfo, SettingValue, TimeRange, Timestamp,
    TrackInfo, TrackKind,
};
use commands::envelope::{CommandOutcome, Generation, RequestId, SessionId};
use commands::{Command, CommandError, ErrorCode, PlaybackState, Stage, WireEvent};
use media_clock::MediaClock;
use media_types::MediaKind;
use playback_engine::event::EngineEventKind;
use playback_engine::{
    CommandReceipt, Engine, EngineOutput, ReceiptResult, SessionCommandKind, SessionConfig,
    SessionError, SessionHandle, SessionState,
};

use crate::demux_host::{MediaFacts, OpenedMedia, open_source, wire_error};
use crate::pipeline_host::{CliFrameConsumer, HostDecoderProvider, RENDER_QUANTUM};

/// The pump bound for "await one observable" (receipts, states) —
/// generous against the fixture's shape (≈10 pumps to full drain).
const AWAIT_PUMPS: usize = 64;

/// The pump bound for run-to-end (the scripted session's `advance`).
pub const RUN_PUMPS: usize = 512;

/// The wire-visible state observation history entry: `(from, to)`.
pub type StateStep = (PlaybackState, PlaybackState);

/// The CLI's engine host. One instance per process (the engine is
/// single-session v0; `lumen_dispatch` is the only writer).
pub struct CliHost {
    engine: Engine,
    handle: Option<SessionHandle>,
    consumer: Option<Arc<CliFrameConsumer>>,
    device: Option<NullDevice>,
    clock: Option<Arc<MediaClock>>,
    facts: Option<Arc<MediaFacts>>,
    source: Option<commands::catalog::types::SourceSpec>,
    allocator: commands::RequestIdAllocator,
    staleness: commands::StalenessFilter,
    /// Receipts drained but not yet matched (bounded by command flow).
    pending_receipts: Vec<CommandReceipt>,
    /// The wire events observed since construction (tests + printing).
    events: Vec<WireEvent>,
    /// The state-transition trail (from the `state-changed` events).
    state_history: Vec<StateStep>,
    /// The last fatal error (wire form) — the honest `open`/`seek`
    /// failure answer.
    last_fatal: Option<CommandError>,
    /// The last track-list snapshot (wire form).
    tracks: Vec<TrackInfo>,
    /// The session's clock anchor (ns).
    anchor_ns: i64,
    /// Session-local settings overrides (the settings-schema seam).
    settings: BTreeMap<String, SettingValue>,
}

impl Default for CliHost {
    fn default() -> Self {
        Self::new()
    }
}

impl CliHost {
    /// An idle host (no session).
    #[must_use]
    pub fn new() -> CliHost {
        CliHost {
            engine: Engine::new(),
            handle: None,
            consumer: None,
            device: None,
            clock: None,
            facts: None,
            source: None,
            allocator: commands::RequestIdAllocator::new(),
            staleness: commands::StalenessFilter::new(),
            pending_receipts: Vec::new(),
            events: Vec::new(),
            state_history: Vec::new(),
            last_fatal: None,
            tracks: Vec::new(),
            anchor_ns: 0,
            settings: BTreeMap::new(),
        }
    }

    // -- observation ------------------------------------------------------

    /// The engine's current state (wire form).
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        session_state_as_wire(self.engine.state())
    }

    /// The live session id, if any.
    #[must_use]
    pub fn session_id(&self) -> Option<SessionId> {
        self.engine.session_id()
    }

    /// The live generation.
    #[must_use]
    pub fn generation(&self) -> Option<Generation> {
        self.engine.generation()
    }

    /// The observed wire events (in causal order; stale ones filtered).
    #[must_use]
    pub fn events(&self) -> &[WireEvent] {
        &self.events
    }

    /// The state-transition trail.
    #[must_use]
    pub fn state_history(&self) -> &[StateStep] {
        &self.state_history
    }

    /// The current position (µs) — the audio-master clock's view
    /// (nanosecond base → µs).
    #[must_use]
    pub fn position_us(&self) -> i64 {
        match self.clock.as_ref() {
            Some(clock) => clock.now().ticks / 1000,
            None => 0,
        }
    }

    /// The harvested facts, when a session is (or was) open.
    #[must_use]
    pub fn facts(&self) -> Option<&Arc<MediaFacts>> {
        self.facts.as_ref()
    }

    /// The engine's pipeline observability counters.
    #[must_use]
    pub fn engine_stats(&self) -> playback_engine::EngineStats {
        self.engine.stats()
    }

    // -- pumping ----------------------------------------------------------

    /// One pump: engine poll, output drain, time advance. See the
    /// module docs — this is the session thread.
    pub fn pump(&mut self) {
        self.engine.poll();
        self.drain_outputs();
        self.advance_time();
        self.surface_video_error();
    }

    /// Pump until `pred` holds (or the bound). Answers whether the
    /// predicate was observed.
    pub fn pump_until(&mut self, mut pred: impl FnMut(&CliHost) -> bool, bound: usize) -> bool {
        for _ in 0..bound {
            if pred(self) {
                return true;
            }
            self.pump();
        }
        pred(self)
    }

    /// Pump until the engine reports `target` (or the bound).
    pub fn pump_until_state(&mut self, target: PlaybackState, bound: usize) -> bool {
        self.pump_until(|host| host.state() == target, bound)
    }

    /// Pump a fixed number of ticks (the `advance` verb).
    pub fn advance(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.pump();
        }
    }

    /// Drain the engine's output hub: receipts stashed for matching,
    /// events converted to wire, staleness-filtered and recorded.
    fn drain_outputs(&mut self) {
        for output in self.engine.take_outputs() {
            match output {
                EngineOutput::Receipt(receipt) => self.pending_receipts.push(receipt),
                EngineOutput::Event(event) => {
                    if event.kind.is_pipeline_scoped() {
                        // Record the machine's trail before the wire
                        // conversion (the engine-private kinds included —
                        // they never reach the wire, doc 05 §4.2).
                        if let EngineEventKind::StateChanged { from, to } = &event.kind {
                            self.state_history
                                .push((session_state_as_wire(*from), session_state_as_wire(*to)));
                        }
                        if let EngineEventKind::Fatal { error } = &event.kind {
                            self.last_fatal = Some(wire_error(error));
                        }
                        if let EngineEventKind::TracksChanged { tracks, selected } = &event.kind {
                            self.tracks = tracks
                                .iter()
                                .zip(selected.iter())
                                .map(|(track, &is_selected)| {
                                    crate::track_info_wire_host(track, is_selected)
                                })
                                .collect();
                        }
                    }
                    if let Some(wire) = event.as_wire_event() {
                        if self.staleness.filter_event(&wire) {
                            self.events.push(wire);
                        }
                    }
                }
            }
        }
    }

    /// Advance media time: render audio quanta (each moves the sample
    /// counter — the clock's truth), ticking the video scheduler once
    /// per quantum. With no audio limb (video-only or `--no-audio`)
    /// the clock is the system monotonic and only the scheduler is
    /// ticked (wall-clock-paced presents).
    fn advance_time(&mut self) {
        let consumer = match self.consumer.as_ref() {
            Some(consumer) => Arc::clone(consumer),
            None => return,
        };
        match self.device.as_mut() {
            Some(device) => {
                let channels = match self.facts
                    .as_ref()
                    .and_then(|facts| facts.facts.iter().find(|fact| fact.pcm.is_some()))
                    .and_then(|fact| fact.pcm)
                {
                    Some((channels, _rate)) => channels as usize,
                    None => 2,
                };
                let mut guard = consumer.lock();
                while device.buffered_frames() >= RENDER_QUANTUM {
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
                    if let Err(error) = device.render(&mut buffer) {
                        guard.note_video_error(error);
                        break;
                    }
                    if let Err(error) = guard.tick_video() {
                        guard.note_video_error(error);
                        break;
                    }
                }
            }
            None => {
                // No audio limb: tick the scheduler a bounded slice per
                // pump (wall-clock presents — see the module docs).
                let mut guard = consumer.lock();
                for _ in 0..16 {
                    if let Err(error) = guard.tick_video() {
                        guard.note_video_error(error);
                        break;
                    }
                }
            }
        }
    }

    /// Surface a present-path failure once (recorded as a wire-visible
    /// fatal marker the runner prints; audio continues — the 2.D1
    /// degrade pattern).
    fn surface_video_error(&mut self) {
        if let Some(consumer) = self.consumer.as_ref()
            && let Some(error) = consumer.take_video_error()
        {
            self.last_fatal = Some(wire_error(&error));
        }
    }

    // -- command routing (the dispatch table's engine half) --------------

    /// Allocate the next request id (doc 05 §4.1 — host-assigned,
    /// echoed by the engine).
    #[must_use]
    pub fn next_request(&mut self) -> RequestId {
        self.allocator.allocate()
    }

    /// Submit one minimal-set command and pump until its receipt
    /// arrives (the truthful-UI correlation point).
    fn submit_and_await(&mut self, kind: SessionCommandKind) -> Result<CommandReceipt, CommandError> {
        let request = self.allocator.allocate();
        let handle = self.handle.as_ref().ok_or_else(|| {
            CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            )
        })?;
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
        for _ in 0..AWAIT_PUMPS {
            self.pump();
            if let Some(position) = self
                .pending_receipts
                .iter()
                .position(|receipt| receipt.request == request)
            {
                let receipt = self.pending_receipts.remove(position);
                return Ok(receipt);
            }
        }
        Err(CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            "the engine never answered the command (bounded wait expired)",
        ))
    }

    /// Translate one receipt into the truthful-UI outcome.
    fn receipt_outcome(&self, receipt: CommandReceipt) -> Result<CommandOutcome<CommandResult>, CommandError> {
        match receipt.result {
            ReceiptResult::Accepted { session, generation } => Ok(CommandOutcome::Accepted {
                session: Some(session),
                generation,
            }),
            ReceiptResult::Completed { session } => {
                let _ = session;
                Ok(CommandOutcome::Completed(CommandResult::Unit))
            }
            ReceiptResult::Rejected { state, reason } => Err(rejection_wire(state, reason)),
        }
    }

    /// The `session.open` path (also the seek reopen).
    fn open_session(
        &mut self,
        source: commands::catalog::types::SourceSpec,
        start_time_us: Option<i64>,
        no_video: bool,
        no_audio: bool,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        if self.engine.session_id().is_some() {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "a session is already open — close it first (source replacement is card 3.A2)",
            ));
        }
        let opened: OpenedMedia =
            open_source(&source, start_time_us, no_video, no_audio)?;
        let facts = Arc::new(opened.facts);
        let anchor_ns = opened.start_position_ns;
        // The audio half (and with it the session clock) exists only
        // when an audio track survived selection.
        let audio_facts: Option<(u32, u32)> = facts
            .facts
            .iter()
            .find(|fact| fact.kind == MediaKind::Audio && fact.pcm.is_some())
            .and_then(|fact| fact.pcm);
        let (audio_limb, device, clock) = match audio_facts {
            Some((channels, rate)) => {
                let AudioSession {
                    limb,
                    device,
                    clock,
                    ..
                } = crate::pipeline_host::open_audio_session(channels, rate, anchor_ns)
                    .map_err(|err| wire_error(&err))?;
                (Some(limb), Some(device), clock)
            }
            None => (
                None,
                None,
                crate::pipeline_host::wall_clock(anchor_ns).map_err(|err| wire_error(&err))?,
            ),
        };
        let consumer = Arc::new(CliFrameConsumer::new(
            audio_limb,
            Arc::clone(&facts),
            Arc::clone(&clock),
        ));
        let provider = Arc::new(HostDecoderProvider::new(Arc::clone(&facts)));
        let config = SessionConfig::new(
            opened.uri,
            Box::new(opened.demuxer),
            provider,
            Arc::clone(&consumer) as Arc<dyn playback_engine::FrameConsumer>,
            Arc::clone(&clock),
        );
        let handle = self
            .engine
            .open(config)
            .map_err(|err| session_error_wire(&err))?;
        self.handle = Some(handle);
        self.consumer = Some(consumer);
        self.device = device;
        self.clock = Some(clock);
        self.facts = Some(facts);
        self.source = Some(source);
        self.anchor_ns = anchor_ns;
        self.tracks.clear();
        // Pump the open to a settled state (Opening → Buffering → …).
        // The truthful answer for the class-A open is `Accepted` —
        // but an immediate open failure (corrupt header, no decodable
        // track) must surface in THIS response, so we pump until the
        // engine leaves `Opening` and check the fatal trail.
        self.pump_until(|host| host.state() != PlaybackState::Opening, AWAIT_PUMPS);
        if let Some(error) = self.last_fatal.take() {
            self.teardown_after_fatal();
            return Err(error);
        }
        let session = self.engine.session_id();
        let generation = self.engine.generation();
        Ok(CommandOutcome::Accepted {
            session,
            generation: generation.unwrap_or(Generation(0)),
        })
    }

    /// After a fatal open failure the engine sits in `Error` with a
    /// dead session — close it so the host is reusable (the wire's
    /// `error → idle` edge).
    fn teardown_after_fatal(&mut self) {
        if self.state() == PlaybackState::Error {
            let _ = self.submit_and_await(SessionCommandKind::Close);
        }
        self.handle = None;
        self.consumer = None;
        self.device = None;
        self.clock = None;
    }

    /// The `playback.seek` composition (see the module docs).
    fn dispatch_seek(
        &mut self,
        target: commands::catalog::types::SeekTarget,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        use commands::catalog::types::SeekTarget;
        let Some(facts) = self.facts.clone() else {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            ));
        };
        let duration_us = facts.duration_us();
        // 1. Resolve the absolute target (µs).
        let target_us: i64 = match target {
            SeekTarget::Absolute { time } => time.0,
            SeekTarget::Relative { delta_ms } => {
                self.position_us().saturating_add(delta_ms.saturating_mul(1000))
            }
            SeekTarget::Percent { fraction } => {
                let Some(duration) = duration_us else {
                    return Err(CommandError::new(
                        Stage::Session,
                        ErrorCode::NotSeekable,
                        "percent seek needs a known duration",
                    ));
                };
                let scaled = (f64::from(duration) * f64::from(fraction)).round() as i64;
                scaled
            }
            SeekTarget::Chapter { index } => {
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::NotFound,
                    format!("no chapter {index}: AVI v0 carries no chapter markers (the chapter card owns this)"),
                ));
            }
        };
        let clamped = target_us.clamp(0, duration_us.unwrap_or(i64::MAX / 4));
        // 2. Stop from the active states (the transient states are
        //    3.C1's — refuse honestly there).
        match self.engine.state() {
            SessionState::Playing | SessionState::Paused => {
                let receipt = self.submit_and_await(SessionCommandKind::Stop)?;
                self.receipt_outcome(receipt)?;
            }
            SessionState::Opening | SessionState::Buffering => {
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::NotReady,
                    "seek while opening/buffering lands with the transient-state stop (card 3.C1)",
                ));
            }
            _ => {}
        }
        // 3. Close (idempotent when settled).
        if self.handle.is_some() {
            let receipt = self.submit_and_await(SessionCommandKind::Close)?;
            self.receipt_outcome(receipt)?;
        }
        self.handle = None;
        self.consumer = None;
        self.device = None;
        self.clock = None;
        // 4. Reopen at the target.
        let source = self
            .source
            .clone()
            .ok_or_else(|| CommandError::new(Stage::Session, ErrorCode::NotReady, "no source"))?;
        let outcome = self.open_session(source, Some(clamped), false, false)?;
        // 5. Let it reach Playing (bounded; short media may go straight
        //    through to Ended — both are success).
        self.pump_until(
            |host| matches!(host.state(), PlaybackState::Playing | PlaybackState::Ended),
            AWAIT_PUMPS,
        );
        if let Some(error) = self.last_fatal.take() {
            return Err(error);
        }
        Ok(outcome)
    }

    /// The class-S snapshot answers.
    fn media_info(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            ));
        };
        let duration_us = facts.duration_us();
        Ok(CommandResult::MediaInfo {
            info: MediaInfo {
                source: self.source.clone().unwrap_or_else(|| {
                    commands::catalog::types::SourceSpec::File {
                        path: std::path::PathBuf::new(),
                    }
                }),
                container: Some(facts.container.clone()),
                title: None,
                duration: match duration_us {
                    Some(us) => Duration::Known { time: Timestamp(us) },
                    None => Duration::Unknown,
                },
                seekable: true,
                is_live: false,
                metadata: BTreeMap::new(),
                tracks: self.tracks.clone(),
                chapters: Vec::<Chapter>::new(),
            },
        })
    }

    fn current_time(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            ));
        };
        let position_us = self.position_us();
        let duration_us = facts.duration_us();
        let fraction = match duration_us {
            Some(duration) if duration > 0 => position_us.clamp(0, duration) as f32 / duration as f32,
            _ => 0.0,
        };
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
        Ok(CommandResult::CurrentTime {
            time: Timestamp(position_us.max(0)),
            position: fraction,
            buffered,
        })
    }

    /// The settings-schema seam: one setting's value + meta.
    fn setting_get(&self, area: &str, key: &str) -> Result<CommandResult, CommandError> {
        let full = format!("{area}.{key}");
        let invalid = |message: String| {
            CommandError::new(Stage::Config, ErrorCode::InvalidArgument, message)
        };
        let def = settings_schema::runtime::schema_meta(&full).map_err(|err| invalid(err.to_string()))?;
        let value = match self.settings.get(&full) {
            Some(stored) => stored.clone(),
            None => default_value(def),
        };
        let mut meta = BTreeMap::new();
        meta.insert("type".to_string(), setting_type_name(def).to_string());
        meta.insert(
            "restart_required".to_string(),
            def.restart_required.to_string(),
        );
        meta.insert("flag".to_string(), def.cli.flag.clone());
        Ok(CommandResult::SettingResult {
            value,
            schema_meta: meta,
        })
    }

    /// The settings-schema seam: validate + store one setting.
    fn setting_set(
        &mut self,
        area: &str,
        key: &str,
        value: SettingValue,
    ) -> Result<CommandResult, CommandError> {
        let full = format!("{area}.{key}");
        let invalid = |message: String| {
            CommandError::new(Stage::Config, ErrorCode::InvalidArgument, message)
        };
        let def = settings_schema::runtime::schema_meta(&full).map_err(|err| invalid(err.to_string()))?;
        let toml_value = setting_value_to_toml(&value);
        settings_schema::runtime::validate_value(&full, &toml_value)
            .map_err(|err| invalid(err.to_string()))?;
        self.settings.insert(full, value.clone());
        Ok(CommandResult::SettingApplied {
            applied: value,
            restart_required: def.restart_required,
        })
    }
}

/// The engine's own wire conversion of its session state (the mapping
/// is the identity — `SessionState::as_wire` — but that method is the
/// engine's; the CLI re-exports through it via this shim to avoid
/// depending on the private field).
fn session_state_as_wire(state: SessionState) -> PlaybackState {
    state.as_wire()
}

/// The engine's `track_info_wire` — reused, not re-implemented
/// (one implementation of the L0→wire track mapping in the tree).
pub(crate) fn track_info_wire_host(
    track: &media_types::Track,
    is_selected: bool,
) -> TrackInfo {
    playback_engine::track_info_wire(track, is_selected)
}

/// A rejection's wire form (doc 05 §2.4's error columns).
fn rejection_wire(state: SessionState, reason: playback_engine::RejectionReason) -> CommandError {
    let stage = Stage::Session;
    let state_name = session_state_as_wire(state);
    match reason {
        playback_engine::RejectionReason::NoSession => CommandError::new(
            stage,
            ErrorCode::NotReady,
            "no session open",
        ),
        playback_engine::RejectionReason::NotReady => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!("the session is still opening (state: {state_name:?})"),
        ),
        playback_engine::RejectionReason::StopFirst => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "stop-or-close from {state_name:?} lands with card 3.C1 (active-close completeness)"
            ),
        ),
        playback_engine::RejectionReason::AlreadySettled => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "the session already settled (state: {state_name:?}); close is the remaining exit"
            ),
        ),
        playback_engine::RejectionReason::RestartNeedsSeek => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "play in {state_name:?} means restart-from-ended — the Phase-3 seek flow (card 3.A1)"
            ),
        ),
        playback_engine::RejectionReason::SourceReplacementPhase3 => CommandError::new(
            stage,
            ErrorCode::NotReady,
            "open on a live session is source replacement (card 3.A2)",
        ),
        playback_engine::RejectionReason::SignalNotRaisedHere => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            format!(
                "a pipeline signal fired in a state whose driver never raises it ({state_name:?}) — bug report"
            ),
        ),
    }
}

/// A session-construction error's wire form.
fn session_error_wire(error: &SessionError) -> CommandError {
    match error {
        SessionError::SessionExists { .. } => CommandError::new(
            Stage::Session,
            ErrorCode::NotReady,
            "a session is already open",
        ),
        SessionError::Config(cause) => CommandError::new(
            Stage::Session,
            ErrorCode::Unsupported,
            cause.to_string(),
        )
        .with_detail("code", "session.no-decodable-track"),
        SessionError::GenerationExhausted(engine) => wire_error(engine),
        SessionError::SessionClosed => CommandError::new(
            Stage::Session,
            ErrorCode::Canceled,
            "the session closed before the command landed",
        ),
        SessionError::Invariant(detail) => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            format!("engine invariant breach: {detail}"),
        ),
    }
}

/// The schema default as a wire value. `product.toml` references are
/// reported as their ref marker (the product-config join is owed to
/// the rebrand-propagation card) `[PROPOSAL]`.
fn default_value(def: &settings_schema::SettingDef) -> SettingValue {
    use settings_schema::model::DefaultValue;
    match &def.default {
        DefaultValue::Literal(value) => toml_to_setting_value(value),
        DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}")),
    }
}

fn toml_to_setting_value(value: &toml::Value) -> SettingValue {
    match value {
        toml::Value::Boolean(value) => SettingValue::Bool(*value),
        toml::Value::Integer(value) => SettingValue::Number(f64::from(*value)),
        toml::Value::Float(value) => SettingValue::Number(*value),
        toml::Value::String(value) => SettingValue::String(value.clone()),
        toml::Value::Array(items) => SettingValue::List(
            items
                .iter()
                .map(|item| match item {
                    toml::Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .collect(),
        ),
        toml::Value::Table(_) => SettingValue::String("(table)".to_string()),
        toml::Value::Datetime(when) => SettingValue::String(when.to_string()),
    }
}

fn setting_value_to_toml(value: &SettingValue) -> toml::Value {
    match value {
        SettingValue::Bool(value) => toml::Value::Boolean(*value),
        SettingValue::Number(value) => {
            if value.fract() == 0.0 && value.abs() < 9.0e15 {
                toml::Value::Integer(*value as i64)
            } else {
                toml::Value::Float(*value)
            }
        }
        SettingValue::String(value) => toml::Value::String(value.clone()),
        SettingValue::List(items) => toml::Value::Array(
            items
                .iter()
                .map(|item| toml::Value::String(item.clone()))
                .collect(),
        ),
    }
}

fn setting_type_name(def: &settings_schema::SettingDef) -> &'static str {
    use settings_schema::model::SettingType;
    match def.ty {
        SettingType::Bool => "bool",
        SettingType::Int => "int",
        SettingType::Float => "float",
        SettingType::String => "string",
        SettingType::Enum => "enum",
        SettingType::Path => "path",
        SettingType::Url => "url",
        SettingType::List(_) => "list",
        SettingType::DurationMs => "duration-ms",
        SettingType::Percent => "percent",
    }
}

/// The engine-info snapshot (class S, data-driven).
#[must_use]
pub fn engine_info() -> EngineInfo {
    EngineInfo {
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        catalog_version: "1".to_string(),
        protocol_version: "unwired (control-protocol is a later card)".to_string(),
        host: "cli".to_string(),
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
            "audio.null-device".to_string(),
            "video.software-compositor".to_string(),
        ],
        plugins: Vec::new(),
        decoders: vec![
            DecoderInfo {
                name: "mjpeg".to_string(),
                kind: TrackKind::Video,
                hardware: false,
            },
            DecoderInfo {
                name: "pcm_s16le".to_string(),
                kind: TrackKind::Audio,
                hardware: false,
            },
        ],
        sessions: Vec::new(),
    }
}

/// The command catalog listing (class S — the catalog as data).
#[must_use]
pub fn command_listings() -> Vec<CommandListing> {
    commands::catalog_data()
        .iter()
        .map(|spec| CommandListing {
            id: spec.id.to_string(),
            class: spec.class,
            since: "1".to_string(),
            deprecated: None,
            requires: Vec::new(),
        })
        .collect()
}
