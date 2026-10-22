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

use audio_output::null::NullDevice;
use commands::catalog::command::{CommandListing, CommandResult};
use commands::catalog::types::{
    Chapter, DecoderInfo, Duration, EngineInfo, MediaInfo, SettingValue, TimeRange, Timestamp,
    TrackInfo, TrackKind,
};
use commands::envelope::{CommandOutcome, Generation, RequestId, SessionId};
use commands::{CommandError, ErrorCode, PlaybackState, Stage, WireEvent};
use media_clock::{MediaClock, MonotonicSource, PlaybackClock};
use media_types::MediaKind;
use playback_engine::event::EngineEventKind;
use playback_engine::{
    CommandReceipt, Engine, EngineOutput, ReceiptResult, SessionCommandKind, SessionConfig,
    SessionError, SessionHandle, SessionState,
};

use crate::demux_host::{MediaFacts, OpenedMedia, open_source, wire_error};
use crate::pipeline_host::{AudioSession, CliFrameConsumer, HostDecoderProvider, RENDER_QUANTUM};

/// The pump bound for "await one observable" (receipts, states) —
/// generous against the fixture's shape (≈10 pumps to full drain).
const AWAIT_PUMPS: usize = 64;

/// The headless frame accounting snapshot (see
/// [`CliHost::headless_accounting`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadlessAccounting {
    /// Frames that reached the consumer (both limbs).
    pub frames_accepted: u64,
    /// Frames the scheduler refused (window full / unschedulable).
    pub frames_refused: u64,
    /// Frames handed to the renderer (presents + repeats).
    pub video_presented: u64,
    /// Scheduler admission refusals.
    pub video_refused: u64,
    /// Frames admitted to the scheduling window.
    pub scheduler_pushed: u64,
    /// Due frames selected for presentation.
    pub scheduler_presented: u64,
    /// Repeat presents within the budget.
    pub scheduler_repeats: u64,
    /// Frames dropped by the late policy.
    pub scheduler_dropped_late: u64,
    /// Frames staged through the renderer's upload path.
    pub staging_frames: u64,
    /// Bytes copied into the staging buffer.
    pub staging_bytes: u64,
    /// Staging buffer copies performed.
    pub staging_copies: u64,
}

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
    /// The open options a seek must preserve on reopen (`--no-video`
    /// / `--no-audio` — the seek composition reopens the SAME source
    /// shape, not the default one).
    open_no_video: bool,
    /// See [`Self::open_no_video`].
    open_no_audio: bool,
    /// Session-local settings overrides (the settings-schema seam).
    settings: BTreeMap<String, SettingValue>,
    /// The reused output-drain buffer (card 3.B4's addendum, D-1):
    /// `take`-out / `drain_outputs_into` / move-back per pump —
    /// steady state allocates nothing on the drain path.
    outbox_scratch: Vec<EngineOutput>,
    /// The injected event-throttle source (card 3.B4): when set,
    /// sessions open with this monotonic source driving the doc 05
    /// §3.4 emission windows instead of real time — the
    /// determinism seam the first-vertical evidence harness uses
    /// (a frozen source keeps the transcripts byte-stable).
    event_throttle: Option<Arc<dyn MonotonicSource>>,
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
            open_no_video: false,
            open_no_audio: false,
            settings: BTreeMap::new(),
            outbox_scratch: Vec::new(),
            event_throttle: None,
        }
    }

    /// Inject the event-throttle source (card 3.B4): every session
    /// opened after this call reads its doc 05 §3.4 emission windows
    /// from `source` instead of real monotonic time. The
    /// first-vertical evidence harness injects a frozen source so
    /// the in-episode `session.buffering` windows and the non-fatal
    /// damage windows are deterministic — production hosts leave it
    /// unset and get the real clock.
    pub fn set_event_throttle(&mut self, source: Arc<dyn MonotonicSource>) {
        self.event_throttle = Some(source);
    }

    // -- observation ------------------------------------------------------

    /// Pump until the engine sits in a settled state (not the
    /// transient `Opening`/`Buffering`) or the bound — the REPL's
    /// post-command pacing policy (see [`crate::run`]). Bounded and
    /// deterministic: every observable is a pure function of the pump
    /// count. Public for the integration suites (the scripted
    /// sessions' open-and-settle step).
    pub fn settle(&mut self) {
        self.pump_pipeline_until(
            |host| {
                !matches!(
                    host.state(),
                    PlaybackState::Opening | PlaybackState::Buffering
                )
            },
            crate::host::AWAIT_PUMPS,
        );
    }

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

    /// The last `tracks.list-ready` snapshot (wire form) — the class-S
    /// `tracks.list` answer.
    #[must_use]
    pub fn track_snapshot(&self) -> Vec<TrackInfo> {
        self.tracks.clone()
    }

    /// The engine's pipeline observability counters.
    #[must_use]
    pub fn engine_stats(&self) -> playback_engine::EngineStats {
        self.engine.stats()
    }

    /// The headless pipeline accounting (the AC's frame evidence):
    /// the consumer's delivery counts plus the video limb's scheduler
    /// and staging measurements. `None` before a session wired a
    /// consumer.
    #[must_use]
    pub fn headless_accounting(&self) -> Option<HeadlessAccounting> {
        let consumer = self.consumer.as_ref()?;
        let guard = consumer.lock();
        let video = guard.video()?;
        let scheduler = video.scheduler_stats();
        let staging = video.staging_stats();
        Some(HeadlessAccounting {
            frames_accepted: guard.frames_accepted,
            frames_refused: guard.frames_refused,
            video_presented: video.presented,
            video_refused: video.refused,
            scheduler_pushed: scheduler.pushed,
            scheduler_presented: scheduler.presented,
            scheduler_repeats: scheduler.repeats,
            scheduler_dropped_late: scheduler.dropped_late,
            staging_frames: staging.frames_staged,
            staging_bytes: staging.bytes_staged,
            staging_copies: staging.staging_copies,
        })
    }

    /// The current surface as a PNG screenshot (2.D1's byte-stable
    /// encoder) — `None` before a video limb exists.
    #[must_use]
    pub fn screenshot_png(&self) -> Option<Result<Vec<u8>, media_types::EngineError>> {
        let consumer = self.consumer.as_ref()?;
        let guard = consumer.lock();
        let video = guard.video()?;
        Some(video.screenshot_png())
    }

    // -- pumping ----------------------------------------------------------

    /// One **pipeline-only** pump: engine poll + output drain + video
    /// error surfacing. NO time advance, NO tail play-out — the
    /// between-commands pacing primitive. Media time moves ONLY via
    /// [`CliHost::advance`] (the `advance` verb), which is what makes
    /// the scripted sessions deterministic and pause/seek landable:
    /// an awaited receipt or a settled open must not consume the
    /// media (the 2.D2 session ACs — pause freezes at 0, seek lands
    /// at its target exactly).
    pub fn pump_pipeline(&mut self) {
        self.engine.poll();
        self.drain_outputs();
        self.surface_video_error();
    }

    /// One FULL pump: pipeline-only steps + time advance + end-tail
    /// play-out. See the module docs — this is the `advance` verb's
    /// tick (the session thread under wall-clock pacing would be
    /// this plus a sleep; headless, `advance N` is N of these).
    pub fn pump(&mut self) {
        self.pump_pipeline();
        self.advance_time();
        self.drain_tail_at_end();
    }

    /// Pipeline-only pump-until (awaits: receipts, settles).
    pub fn pump_pipeline_until(
        &mut self,
        mut pred: impl FnMut(&CliHost) -> bool,
        bound: usize,
    ) -> bool {
        for _ in 0..bound {
            if pred(self) {
                return true;
            }
            self.pump_pipeline();
        }
        pred(self)
    }

    /// Pump until `pred` holds (or the bound). Answers whether the
    /// predicate was observed. FULL pumps (time advances) — the
    /// `advance` verb's driver; awaits use [`CliHost::pump_pipeline_until`].
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
    /// The buffer is REUSED across pumps (D-1): take → clear →
    /// `drain_outputs_into` → drain while processing → move back —
    /// the Vec's allocation survives, steady state allocates
    /// nothing per drain.
    fn drain_outputs(&mut self) {
        let mut outputs = std::mem::take(&mut self.outbox_scratch);
        outputs.clear();
        self.engine.drain_outputs_into(&mut outputs);
        for output in outputs.drain(..) {
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
                            let facts = self.facts.as_ref();
                            self.tracks = tracks
                                .iter()
                                .zip(selected.iter())
                                .map(|(track, &is_selected)| {
                                    enrich_track_info(
                                        track_info_wire_host(track, is_selected),
                                        facts.and_then(|facts| facts.fact_for(track.id)),
                                    )
                                })
                                .collect();
                        }
                    }
                    if let Some(wire) = event.as_wire_event()
                        && self.staleness.filter_event(&wire)
                    {
                        self.events.push(wire);
                    }
                }
            }
        }
        self.outbox_scratch = outputs;
    }

    /// Advance media time: render audio quanta (each moves the sample
    /// counter — the clock's truth), ticking the video scheduler once
    /// per quantum. With no audio limb (video-only or `--no-audio`)
    /// the clock is the system monotonic and only the scheduler is
    /// ticked (wall-clock-paced presents).
    ///
    /// **Paused clock = the device pulls nothing.** The engine freezes
    /// the session clock on `pause`/underrun (`Effect::FreezeClock`);
    /// rendering a quantum anyway would (a) spin forever — a render
    /// under a frozen `MediaClock` still consumes the ring but the
    /// loop's `buffered_frames` predicate would never fall below the
    /// quantum — and (b) desynchronize: `MediaClock::resume` re-anchors
    /// at the frozen position, so audio consumed while paused is
    /// positionally skipped. Skipping renders while the clock is
    /// frozen keeps the ring's content and the clock's position in
    /// lockstep; `resume` continues exactly where the pause landed
    /// (the 2.C3 pause test's own pattern).
    fn advance_time(&mut self) {
        if self.clock.as_ref().is_some_and(|clock| clock.is_paused()) {
            return;
        }
        let consumer = match self.consumer.as_ref() {
            Some(consumer) => Arc::clone(consumer),
            None => return,
        };
        let channels = self.audio_channels();
        match self.device.as_mut() {
            Some(device) => {
                let mut guard = consumer.lock();
                while device.buffered_frames() >= RENDER_QUANTUM {
                    // Tick BEFORE the render: the scheduler's slot
                    // decision must see the clock as it stands at the
                    // slot boundary (the vsync-then-advance order — a
                    // frame presents exactly when its slot arrives,
                    // never one quantum late).
                    if let Err(error) = guard.tick_video() {
                        guard.note_video_error(error);
                        break;
                    }
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
                    if let Err(error) = device.render(&mut buffer) {
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

    /// The audio limb's channel count (render-buffer sizing; the
    /// harvested PCM facts, stereo default).
    fn audio_channels(&self) -> usize {
        match self
            .facts
            .as_ref()
            .and_then(|facts| facts.facts.iter().find(|fact| fact.pcm.is_some()))
            .and_then(|fact| fact.pcm)
        {
            Some((channels, _rate)) => channels as usize,
            None => 2,
        }
    }

    /// **The end-of-media tail play-out.** The engine stops the
    /// session clock when the *pipeline* drains (`Drained → Ended`),
    /// but the null ring may still hold the audio tail — the decode
    /// side runs ahead of the device by up to the ring's content. A
    /// real device keeps pulling its callback until the buffer empties
    /// (then silence); the null equivalent is: resume the clock, render
    /// the residue (each render moving the counter — the clock's
    /// truth), then freeze again. The observable position reaches the
    /// media end exactly; the final render is sized to the residue so
    /// no sub-quantum remainder is stranded.
    fn drain_tail_at_end(&mut self) {
        if self.engine.state() != SessionState::Ended {
            return;
        }
        let channels = self.audio_channels();
        let consumer = match self.consumer.as_ref() {
            Some(consumer) => Arc::clone(consumer),
            None => return,
        };
        let clock = match self.clock.as_ref() {
            Some(clock) => Arc::clone(clock),
            None => return,
        };
        let Some(device) = self.device.as_mut() else {
            return;
        };
        if device.buffered_frames() == 0 {
            return;
        }
        clock.resume();
        let mut guard = consumer.lock();
        while device.buffered_frames() > 0 {
            let take = device.buffered_frames().min(RENDER_QUANTUM);
            // Same vsync-then-advance order as `advance_time`.
            if let Err(error) = guard.tick_video() {
                guard.note_video_error(error);
                break;
            }
            let mut buffer = vec![0.0_f32; take * channels];
            if let Err(error) = device.render(&mut buffer) {
                guard.note_video_error(error);
                break;
            }
        }
        clock.pause();
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
    pub(crate) fn submit_and_await(
        &mut self,
        kind: SessionCommandKind,
    ) -> Result<CommandReceipt, CommandError> {
        let request = self.allocator.allocate();
        let handle = self.handle.as_ref().ok_or_else(|| {
            CommandError::new(Stage::Session, ErrorCode::NotReady, "no session open")
        })?;
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
        for _ in 0..AWAIT_PUMPS {
            self.pump_pipeline();
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
    pub(crate) fn receipt_outcome(
        &self,
        receipt: CommandReceipt,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        match receipt.result {
            ReceiptResult::Accepted {
                session,
                generation,
            } => Ok(CommandOutcome::Accepted {
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
    pub(crate) fn open_session(
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
        let opened: OpenedMedia = open_source(&source, start_time_us, no_video, no_audio)?;
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
        let mut provider = HostDecoderProvider::new(Arc::clone(&facts));
        // The drain-gate probe: the PCM decoder's `Drained` answer
        // waits on the null device's ring actually emptying (see
        // DrainGatedDecoder — headless time moves only via `advance`).
        provider.attach_consumer(Arc::clone(&consumer));
        let provider = Arc::new(provider);
        let mut config = SessionConfig::new(
            opened.uri,
            Box::new(opened.demuxer),
            provider,
            Arc::clone(&consumer) as Arc<dyn playback_engine::FrameConsumer>,
            Arc::clone(&clock),
        );
        if let Some(source) = self.event_throttle.clone() {
            config.throttle = playback_engine::EventPolicy {
                tick: playback_engine::TimeTickPolicy::default_policy(),
                throttle: source,
            };
        }
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
        self.open_no_video = no_video;
        self.open_no_audio = no_audio;
        self.tracks.clear();
        // Pump the open to a settled state (Opening → Buffering → …).
        // The truthful answer for the class-A open is `Accepted` —
        // but an immediate open failure (corrupt header, no decodable
        // track) must surface in THIS response, so we pump until the
        // engine leaves `Opening` and check the fatal trail.
        self.pump_pipeline_until(|host| host.state() != PlaybackState::Opening, AWAIT_PUMPS);
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
        self.teardown_session_scoped();
    }

    /// Clear every session-scoped host half (after a settled Close or
    /// a fatal): the snapshot answers (`time`, `info`) then honestly
    /// refuse with not-ready instead of answering from a dead
    /// session's leftover clock/facts.
    fn teardown_session_scoped(&mut self) {
        self.handle = None;
        self.consumer = None;
        self.device = None;
        self.clock = None;
        self.facts = None;
        self.source = None;
        self.anchor_ns = 0;
        self.tracks.clear();
        self.pending_receipts.clear();
    }

    /// A settled Close: the engine went to Idle — drop the host's
    /// session-scoped halves so later snapshots refuse honestly.
    pub(crate) fn session_closed(&mut self) {
        self.teardown_session_scoped();
    }

    /// The `playback.seek` composition (see the module docs).
    pub(crate) fn dispatch_seek(
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
            SeekTarget::Relative { delta_ms } => self
                .position_us()
                .saturating_add(delta_ms.saturating_mul(1000)),
            SeekTarget::Percent { fraction } => {
                let Some(duration) = duration_us else {
                    return Err(CommandError::new(
                        Stage::Session,
                        ErrorCode::NotSeekable,
                        "percent seek needs a known duration",
                    ));
                };
                ((duration as f64) * f64::from(fraction)).round() as i64
            }
            SeekTarget::Chapter { index } => {
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::NotFound,
                    format!(
                        "no chapter {index}: AVI v0 carries no chapter markers (the chapter card owns this)"
                    ),
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
        let outcome = self.open_session(
            source,
            Some(clamped),
            self.open_no_video,
            self.open_no_audio,
        )?;
        // 5. Let it reach Playing (bounded; short media may go straight
        //    through to Ended — both are success).
        self.pump_pipeline_until(
            |host| matches!(host.state(), PlaybackState::Playing | PlaybackState::Ended),
            AWAIT_PUMPS,
        );
        if let Some(error) = self.last_fatal.take() {
            return Err(error);
        }
        Ok(outcome)
    }

    /// The class-S snapshot answers.
    pub(crate) fn media_info(&self) -> Result<CommandResult, CommandError> {
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
                    Some(us) => Duration::Known {
                        time: Timestamp(us),
                    },
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

    /// The `media.get-duration` snapshot (class S).
    pub(crate) fn duration(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotReady,
                "no session open",
            ));
        };
        Ok(CommandResult::Duration {
            duration: match facts.duration_us() {
                Some(us) => Duration::Known {
                    time: Timestamp(us),
                },
                None => Duration::Unknown,
            },
        })
    }

    pub(crate) fn current_time(&self) -> Result<CommandResult, CommandError> {
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
            Some(duration) if duration > 0 => {
                position_us.clamp(0, duration) as f32 / duration as f32
            }
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
    pub(crate) fn setting_get(&self, area: &str, key: &str) -> Result<CommandResult, CommandError> {
        let full = format!("{area}.{key}");
        let invalid =
            |message: String| CommandError::new(Stage::Config, ErrorCode::InvalidArgument, message);
        let def =
            settings_schema::runtime::schema_meta(&full).map_err(|err| invalid(err.to_string()))?;
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
    pub(crate) fn setting_set(
        &mut self,
        area: &str,
        key: &str,
        value: SettingValue,
    ) -> Result<CommandResult, CommandError> {
        let full = format!("{area}.{key}");
        let invalid =
            |message: String| CommandError::new(Stage::Config, ErrorCode::InvalidArgument, message);
        let def =
            settings_schema::runtime::schema_meta(&full).map_err(|err| invalid(err.to_string()))?;
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
pub(crate) fn track_info_wire_host(track: &media_types::Track, is_selected: bool) -> TrackInfo {
    playback_engine::track_info_wire(track, is_selected)
}

/// Fold the open path's harvested facts into one wire track row: the
/// L0 [`Track`] deliberately carries no codec parameters, but the
/// demuxer's `strf` did — `media.get-info` owes the caller the
/// container's own declaration (dimensions, channel count, rates).
fn enrich_track_info(
    mut info: TrackInfo,
    fact: Option<&crate::demux_host::TrackFact>,
) -> TrackInfo {
    let Some(fact) = fact else {
        return info;
    };
    if let Some((width, height)) = fact.dimensions {
        info.video = Some(commands::catalog::types::VideoTrackInfo {
            width,
            height,
            frame_rate: fact.frame_ns.map(|ns| (1.0e9 / ns as f64) as f32),
            bit_depth: None,
            hdr: false,
        });
    }
    if let Some((channels, rate)) = fact.pcm {
        info.audio = Some(commands::catalog::types::AudioTrackInfo {
            channels: channels.min(u16::MAX as u32) as u16,
            sample_rate: Some(rate),
            bit_depth: None,
            codec_params: None,
        });
    }
    info
}

/// A rejection's wire form (doc 05 §2.4's error columns).
fn rejection_wire(state: SessionState, reason: playback_engine::RejectionReason) -> CommandError {
    let stage = Stage::Session;
    let state_name = session_state_as_wire(state);
    match reason {
        playback_engine::RejectionReason::NoSession => {
            CommandError::new(stage, ErrorCode::NotReady, "no session open")
        }
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
        playback_engine::RejectionReason::EndedNeedsWireEdge => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "seek in {state_name:?} needs the doc 05 §3.3 ended→buffering wire edge (Lead-owned change)"
            ),
        ),
        playback_engine::RejectionReason::MissingPayload => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            "a seek command arrived without its target payload — caller bug",
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
        SessionError::Config(cause) => {
            CommandError::new(Stage::Session, ErrorCode::Unsupported, cause.to_string())
                .with_detail("code", "session.no-decodable-track")
        }
        SessionError::GenerationExhausted(engine) => wire_error(engine),
        SessionError::SessionClosed => CommandError::new(
            Stage::Session,
            ErrorCode::Canceled,
            "the session closed before the command landed",
        ),
        SessionError::InboxBusy => CommandError::new(
            Stage::Session,
            ErrorCode::ResourceLimit,
            "the session inbox is full — poll the engine, then retry",
        )
        .with_detail("limit", "inbox-capacity"),
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
        toml::Value::Integer(value) => SettingValue::Number(*value as f64),
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
                backend: None,
                active: false,
            },
            DecoderInfo {
                name: "pcm_s16le".to_string(),
                kind: TrackKind::Audio,
                hardware: false,
                backend: None,
                active: false,
            },
        ],
        sessions: Vec::new(),
    }
}

/// The command catalog listing (class S — the catalog as data).
#[must_use]
pub fn command_listings() -> Vec<CommandListing> {
    commands::catalog::catalog_data()
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
