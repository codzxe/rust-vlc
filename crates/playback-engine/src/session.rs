//! The engine and its session (card 2.C2): the single-writer
//! orchestrator that owns the state machine, mints generations, and
//! emits the event stream (doc 03 §4.6(a)).
//!
//! # Threading model, v0 (doc 03 §4.6(c)/§9)
//!
//! Doc 03 gives the session its own thread with a command inbox and
//! an event bus. The **minimal machine** keeps the single-writer
//! discipline without spawning anything: every mutation happens
//! behind `&mut self` on [`Engine::poll`], so invariant S1 ("state
//! transitions occur only on the session thread") holds *by
//! construction* — there is no second writer to exclude. Commands
//! arrive through a std mpsc inbox ([`SessionHandle`]), which is the
//! only cross-thread edge; the thread-per-session spawn, the demux
//! and decode workers, and the event broadcast bus land with cards
//! 2.E1/3.B3. Until then the host's poll loop *is* the session
//! thread.
//!
//! # Command application points (doc 03 §4.6(b))
//!
//! `poll` drains the inbox first, applying every command through the
//! transition table — so commands are always applied *between*
//! pipeline ticks, never mid-tick — then advances the pipeline by one
//! bounded tick. A `stop`/`close` (the gen-bumping commands of the
//! minimal set) mints its pipeline generation at accept time, exactly
//! as doc 05 §4.2 requires, and the receipt carries the new
//! generation.
//!
//! # Output ordering (doc 05 §4.3)
//!
//! Outputs ([`EngineOutput`]) accumulate in the engine's
//! [`OutputHub`] until the host drains them with
//! [`Engine::take_outputs`]. The receipt for a command precedes any
//! event the command caused; `playback.error` precedes
//! `state-changed → error`; `playback.end-of-media` precedes
//! `state-changed → ended`; `tracks.list-ready` precedes the session's
//! first `state-changed → playing`. Superseded pipeline events
//! (emitted under an older generation than the live one) are
//! **suppressed** at drain time — the engine-side half of doc 05
//! §4.2's staleness rule — while receipts always survive.

use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};

use commands::CommandId;
use commands::envelope::{Generation, RequestId, SessionId};
use media_clock::{MediaClock, PlaybackClock};
use media_types::{EngineError, GenerationId, SourceUri};
use plugin_api::SeekSupport;

use crate::epoch::{CancelToken, GenerationPair, PipelineGeneration, SessionEpoch, TokenMinter};
use crate::error::{SessionConfigError, SessionError};
use crate::event::{EngineEvent, EngineEventKind};
use crate::pipeline::{
    DecoderProvider, FrameConsumer, Pipeline, QueuePlan, SeekPreroll, TickBudget, TickReport,
};
use crate::seek::{
    InFlightSeek, ResolvedSeek, SeekTarget, complete_seek, nanos_i64, not_seekable, pipeline_seek,
    resolve_target,
};
use crate::state::{
    Effect, PipelineSignal, RejectionReason, SessionInput, SessionState, TRANSITIONS, Transition,
    TransitionRejected, apply_input, translate_pipeline_signal,
};

/// The outbox capacity (events; receipts are never dropped for
/// capacity). `[PROPOSAL: doc 05 §3.4's burst bound is 1000 events/s
/// to a host; the outbox holds one drain interval's worth — hosts
/// that never drain see oldest events dropped and counted rather
/// than unbounded memory]`.
pub const OUTBOX_CAPACITY: usize = 1024;

/// One client command kind, engine-side (the state-machine commands:
/// play/pause/stop/close plus card 3.A1's seek; `set-rate`/track
/// selection are Phase 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommandKind {
    /// `playback.play`.
    Play,
    /// `playback.pause`.
    Pause,
    /// `playback.stop` (gen-bumping).
    Stop,
    /// `session.close` (class S, gen-bumping, idempotent).
    Close,
    /// `playback.seek` (class A, gen-bumping; card 3.A1, doc 03 §6) —
    /// the seven-step seek flow. The target rides the command as its
    /// payload (a payload-less seek is the typed `MissingPayload`
    /// rejection at the command boundary).
    Seek,
}

impl SessionCommandKind {
    /// The wire catalog id (doc 05 §2.1) — the reconciliation point
    /// between this enum and the command layer's catalog data.
    #[must_use]
    pub fn as_command_id(self) -> CommandId {
        match self {
            SessionCommandKind::Play => CommandId::PlaybackPlay,
            SessionCommandKind::Pause => CommandId::PlaybackPause,
            SessionCommandKind::Stop => CommandId::PlaybackStop,
            SessionCommandKind::Close => CommandId::SessionClose,
            SessionCommandKind::Seek => CommandId::PlaybackSeek,
        }
    }

    /// The state-machine input the command maps to.
    #[must_use]
    pub fn as_input(self) -> SessionInput {
        match self {
            SessionCommandKind::Play => SessionInput::Play,
            SessionCommandKind::Pause => SessionInput::Pause,
            SessionCommandKind::Stop => SessionInput::Stop,
            SessionCommandKind::Close => SessionInput::Close,
            SessionCommandKind::Seek => SessionInput::Seek,
        }
    }
}

/// One queued command (the inbox item). Not `Eq`: a seek target may
/// carry a float fraction (the wire `SeekTarget` likewise).
#[derive(Debug, Clone, Copy, PartialEq)]
struct SessionCommand {
    request: RequestId,
    kind: SessionCommandKind,
    /// The seek target, when the command is a `playback.seek` — the
    /// payload rides the command into the executor's context (the
    /// table itself stays pure data).
    seek: Option<SeekTarget>,
}

/// The result half of a command receipt: the truthful-UI split
/// (doc 05 §1.4) — `Accepted` (class A, completion arrives as
/// events) vs `Completed` (class S, this response is the proof) vs
/// `Rejected` (typed, inapplicable here and now).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptResult {
    /// Class A: accepted under this generation; the completion
    /// evidence arrives as events (doc 05 §1.4 C3).
    Accepted {
        /// The session that accepted it.
        session: SessionId,
        /// The generation in force **after** accept (the NEW one for
        /// gen-bumping commands — doc 05 §4.2).
        generation: Generation,
    },
    /// Class S: the engine confirms completion in this receipt (no
    /// event required — doc 05 §1.4 C2).
    Completed {
        /// The session that completed it.
        session: SessionId,
    },
    /// The command is not applicable in the current state — a typed
    /// result, never an error crash (doc 03 §4.6).
    Rejected {
        /// The state at rejection time.
        state: SessionState,
        /// Why (see [`RejectionReason`]).
        reason: RejectionReason,
    },
}

impl ReceiptResult {
    /// The generation an `Accepted` receipt speaks for, if any
    /// (mirrors `CommandOutcome::accepted_generation`, doc 05 §1.6).
    #[must_use]
    pub fn accepted_generation(&self) -> Option<Generation> {
        match self {
            ReceiptResult::Accepted { generation, .. } => Some(*generation),
            _ => None,
        }
    }
}

/// The receipt for one submitted command, correlated by request id
/// (doc 05 §4.1 — the engine echoes, it never interprets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandReceipt {
    /// The echoed request id.
    pub request: RequestId,
    /// Which command it was.
    pub command: SessionCommandKind,
    /// The outcome.
    pub result: ReceiptResult,
}

/// One engine output: a command receipt or an event, in causal order
/// (doc 05 §4.3 — one ordered stream per host).
#[derive(Debug, Clone, PartialEq)]
pub enum EngineOutput {
    /// The response to a submitted command.
    Receipt(CommandReceipt),
    /// An engine → host event.
    Event(EngineEvent),
}

/// Everything a host needs to drive one playback (doc 03 §4.6(b)
/// `SessionHandle`): submit commands; the session's outputs arrive
/// through [`Engine::take_outputs`].
///
/// The handle is cheap to clone and `Send`; submitting after the
/// session closed answers the typed [`SessionError::SessionClosed`]
/// — the command-side staleness rejection (a handle outliving its
/// session's epoch).
#[derive(Debug, Clone)]
pub struct SessionHandle {
    epoch: SessionEpoch,
    tx: Sender<SessionCommand>,
}

impl SessionHandle {
    /// The session's epoch as its wire id (doc 05 §4.2 mapping).
    #[must_use]
    pub fn session_id(&self) -> SessionId {
        self.epoch.as_session_id()
    }

    /// Submit a command (doc 05 §4.1: `request` is the host-assigned
    /// id, echoed in the receipt).
    ///
    /// # Errors
    /// [`SessionError::SessionClosed`] when the target session is
    /// gone (stale handle) — never a silent drop.
    pub fn submit(&self, request: RequestId, kind: SessionCommandKind) -> Result<(), SessionError> {
        self.tx
            .send(SessionCommand {
                request,
                kind,
                seek: None,
            })
            .map_err(|_| SessionError::SessionClosed)
    }

    /// Submit `playback.play`.
    ///
    /// # Errors
    /// As [`SessionHandle::submit`].
    pub fn play(&self, request: RequestId) -> Result<(), SessionError> {
        self.submit(request, SessionCommandKind::Play)
    }

    /// Submit `playback.pause`.
    ///
    /// # Errors
    /// As [`SessionHandle::submit`].
    pub fn pause(&self, request: RequestId) -> Result<(), SessionError> {
        self.submit(request, SessionCommandKind::Pause)
    }

    /// Submit `playback.stop`.
    ///
    /// # Errors
    /// As [`SessionHandle::submit`].
    pub fn stop(&self, request: RequestId) -> Result<(), SessionError> {
        self.submit(request, SessionCommandKind::Stop)
    }

    /// Submit `session.close`.
    ///
    /// # Errors
    /// As [`SessionHandle::submit`].
    pub fn close(&self, request: RequestId) -> Result<(), SessionError> {
        self.submit(request, SessionCommandKind::Close)
    }

    /// Submit `playback.seek` with its target (card 3.A1; doc 05
    /// §2.4 — the payload rides the command, and the class-A accept
    /// receipt carries the post-bump generation; the completion
    /// arrives as the `Seeked`/`SeekFailed` terminal events).
    ///
    /// # Errors
    /// As [`SessionHandle::submit`].
    pub fn seek(&self, request: RequestId, target: SeekTarget) -> Result<(), SessionError> {
        self.tx
            .send(SessionCommand {
                request,
                kind: SessionCommandKind::Seek,
                seek: Some(target),
            })
            .map_err(|_| SessionError::SessionClosed)
    }
}

/// What a session is opened with (card 2.C2: everything injectable —
/// the engine owns no concrete demuxer, decoder, sink or clock
/// construction). Not `Clone`: the demuxer and the clock ownership
/// move into the session on open (a second open needs a second
/// config).
pub struct SessionConfig {
    /// The source being opened (carried for events and error
    /// context; access resolution is the demuxer impl's business).
    pub source: SourceUri,
    /// The demux stage (plugin-api contract).
    pub demuxer: Box<dyn plugin_api::Demuxer>,
    /// The decoder seam (registry, future; fakes in tests).
    pub decoders: Arc<dyn DecoderProvider>,
    /// The sink seam (audio pump / video scheduler, cards 2.C3/2.C4).
    pub consumer: Arc<dyn FrameConsumer>,
    /// The session clock (v0: audio-master over an injected source or
    /// the explicit system-monotonic fallback — construction is the
    /// host's, per the media-clock injection doctrine).
    pub clock: Arc<MediaClock>,
    /// Queue caps and the engine's watermark policy.
    pub queues: QueuePlan,
    /// The per-tick work bound.
    pub budget: TickBudget,
}

impl SessionConfig {
    /// A config with the given stage objects and doc-default queue
    /// plan and tick budget.
    #[must_use]
    pub fn new(
        source: SourceUri,
        demuxer: Box<dyn plugin_api::Demuxer>,
        decoders: Arc<dyn DecoderProvider>,
        consumer: Arc<dyn FrameConsumer>,
        clock: Arc<MediaClock>,
    ) -> SessionConfig {
        SessionConfig {
            source,
            demuxer,
            decoders,
            consumer,
            clock,
            queues: QueuePlan::default(),
            budget: TickBudget::default(),
        }
    }
}

/// The engine's output buffer with its staleness and overflow policy
/// (see the module docs).
#[derive(Debug)]
struct OutputHub {
    queue: VecDeque<EngineOutput>,
    capacity: usize,
    dropped_events: u64,
    suppressed_stale: u64,
}

impl OutputHub {
    fn new(capacity: usize) -> OutputHub {
        OutputHub {
            queue: VecDeque::new(),
            capacity: capacity.max(1),
            dropped_events: 0,
            suppressed_stale: 0,
        }
    }

    /// Append one output. At capacity the **oldest event** is dropped
    /// and counted — receipts are pinned (responses are never
    /// suppressed, doc 05 §4.3); if the buffer holds only receipts it
    /// grows past capacity (receipt volume is host-bounded).
    fn push(&mut self, output: EngineOutput) {
        if self.queue.len() >= self.capacity
            && let Some(oldest_event) = self
                .queue
                .iter()
                .position(|item| matches!(item, EngineOutput::Event(_)))
        {
            self.queue.remove(oldest_event);
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
        self.queue.push_back(output);
    }

    /// Drain everything, suppressing pipeline-scoped events emitted
    /// under generations older than `current` (doc 05 §4.2 — the
    /// engine-side staleness rule: superseded work is silent) — with
    /// one exemption: the two seek terminals. A `Seeked`/`SeekFailed`
    /// already emitted for a completed generation survives a later
    /// mint that lands before the host drains (doc 05 §4.3 invariant
    /// 4: exactly one terminal per completed generation).
    /// Superseded seeks stay silent the honest way — their slot was
    /// replaced before any terminal was emitted, so there is nothing
    /// in the buffer to swallow.
    fn drain(&mut self, current: &GenerationPair) -> Vec<EngineOutput> {
        let live = current.pipeline.0;
        let mut kept = Vec::with_capacity(self.queue.len());
        while let Some(item) = self.queue.pop_front() {
            match &item {
                EngineOutput::Receipt(_) => kept.push(item),
                EngineOutput::Event(event) => {
                    let seek_terminal = matches!(
                        event.kind,
                        EngineEventKind::Seeked { .. } | EngineEventKind::SeekFailed { .. }
                    );
                    if event.generation.0 < live && !seek_terminal {
                        self.suppressed_stale = self.suppressed_stale.saturating_add(1);
                    } else {
                        kept.push(item);
                    }
                }
            }
        }
        kept
    }
}

/// Observability counters for the output path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EngineStats {
    /// Events dropped for outbox capacity (oldest-first; receipts
    /// are never dropped).
    pub outbox_dropped_events: u64,
    /// Pipeline-scoped events suppressed at drain as superseded
    /// (doc 05 §4.2).
    pub suppressed_stale_events: u64,
    /// Frames delivered to the sink seam (lifetime of the engine,
    /// across sessions).
    pub frames_delivered: u64,
    /// Frames dropped at the sink for stale generations (doc 03 §5
    /// checking point 5). Zero by construction in v0's single-writer
    /// ticks — the counter is the checkpoint the threaded future
    /// (2.E1/3.B3) will exercise.
    pub stale_frames_dropped: u64,
    /// Demuxer-marked discontinuities observed (the re-anchor policy
    /// is Phase 3, card 3.B1; the count is the observable fact).
    pub demux_discontinuities: u64,
}

/// Per-session pipeline observability (merged into [`EngineStats`]).
/// Engine-level totals survive session destruction; session-level
/// totals die with the session (the single writer accumulates them
/// in [`Session::tick_step`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PipelineTotals {
    frames_delivered: u64,
    stale_frames_dropped: u64,
    demux_discontinuities: u64,
}

impl PipelineTotals {
    /// Fold one tick's report into the totals.
    fn merge_tick(&mut self, report: &TickReport) {
        self.frames_delivered = self.frames_delivered.saturating_add(report.frames);
        self.stale_frames_dropped = self
            .stale_frames_dropped
            .saturating_add(report.stale_frames);
        self.demux_discontinuities = self
            .demux_discontinuities
            .saturating_add(report.discontinuities);
    }

    /// Fold another totals snapshot into this one (session close).
    fn merge(&mut self, other: &PipelineTotals) {
        self.frames_delivered = self.frames_delivered.saturating_add(other.frames_delivered);
        self.stale_frames_dropped = self
            .stale_frames_dropped
            .saturating_add(other.stale_frames_dropped);
        self.demux_discontinuities = self
            .demux_discontinuities
            .saturating_add(other.demux_discontinuities);
    }
}

/// The engine (card 2.C2): mints session epochs, hosts **one**
/// session, and serializes its outputs.
///
/// Single-session v0 `[PROPOSAL: multi-session hosting arrives with
/// the hosts that need it; play-new-while-playing replacement is
/// card 3.A2 — v0 refuses a second open on a live session honestly
/// instead of silently replacing]`.
pub struct Engine {
    next_epoch: u64,
    seq: Arc<AtomicU64>,
    session: Option<Session>,
    outbox: OutputHub,
    last_pair: Option<GenerationPair>,
    pipeline_totals: PipelineTotals,
}

impl fmt::Debug for Engine {
    // Manual on purpose: `Session` carries trait objects (`Demuxer`,
    // `Decoder`, seams) with no `Debug` supertrait — the debug form
    // reports the observable surface, not the pipeline internals.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine")
            .field("state", &self.state())
            .field("session", &self.session_id())
            .field("generation", &self.generation())
            .field("stats", &self.stats())
            .finish_non_exhaustive()
    }
}

impl Engine {
    /// An engine with no session (state `Idle`).
    #[must_use]
    pub fn new() -> Engine {
        Engine::with_outbox_capacity(OUTBOX_CAPACITY)
    }

    /// An engine with an explicit outbox capacity (tests).
    #[must_use]
    pub fn with_outbox_capacity(capacity: usize) -> Engine {
        Engine {
            next_epoch: SessionEpoch::FIRST.0,
            seq: Arc::new(AtomicU64::new(0)),
            session: None,
            outbox: OutputHub::new(capacity),
            last_pair: None,
            pipeline_totals: PipelineTotals::default(),
        }
    }

    /// Open a session: mint the epoch, construct the session, and
    /// apply the `Idle → Opening` transition (doc 03 §4.6 — the
    /// engine-side `Open(source)`; the source travels in `config`).
    ///
    /// # Errors
    /// [`SessionError::SessionExists`] if a live session exists;
    /// [`SessionError::GenerationExhausted`] if the epoch counter is
    /// spent.
    pub fn open(&mut self, config: SessionConfig) -> Result<SessionHandle, SessionError> {
        if let Some(session) = &self.session {
            return Err(SessionError::SessionExists {
                session: session.epoch.as_session_id(),
            });
        }
        let epoch = SessionEpoch(self.next_epoch);
        let next = self
            .next_epoch
            .checked_add(1)
            .ok_or_else(SessionError::generation_exhausted_boxed)?;
        self.next_epoch = next;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut session = Session::new(epoch, rx, config, Arc::clone(&self.seq));
        // The pinned `Idle -- Open --> Opening` row: it cannot be
        // rejected (freshly constructed state), but the Result is
        // honored honestly rather than unwrapped. Its `state-changed →
        // opening` announcement goes to the host like every other
        // transition (doc 05 §3.2 — never coalesced).
        let context = InputContext::default();
        let announcements = match session.apply(SessionInput::Open, context) {
            Ok(outputs) => outputs,
            Err(rejection) => return Err(SessionError::Invariant(format!("{rejection}"))),
        };
        for output in announcements {
            self.outbox.push(output);
        }
        self.last_pair = Some(session.pair());
        self.session = Some(session);
        Ok(SessionHandle { epoch, tx })
    }

    /// The observed playback state (`Idle` with no session — doc 05
    /// §3.3's engine view).
    #[must_use]
    pub fn state(&self) -> SessionState {
        self.session
            .as_ref()
            .map_or(SessionState::Idle, |s| s.state)
    }

    /// The live session's wire id, if any.
    #[must_use]
    pub fn session_id(&self) -> Option<SessionId> {
        self.session.as_ref().map(|s| s.epoch.as_session_id())
    }

    /// The live (or last) pipeline generation in wire form.
    #[must_use]
    pub fn generation(&self) -> Option<Generation> {
        self.current_pair().map(GenerationPair::wire_generation)
    }

    /// The live session's cancel token (doc 03 §5) for stage wiring
    /// (the audio pump and video scheduler of cards 2.C3/2.C4).
    #[must_use]
    pub fn cancel_token(&self) -> Option<CancelToken> {
        self.session.as_ref().map(|s| s.minter.token())
    }

    /// The output-path counters.
    #[must_use]
    pub fn stats(&self) -> EngineStats {
        let mut totals = self.pipeline_totals;
        if let Some(session) = &self.session {
            totals.merge(&session.totals);
        }
        EngineStats {
            outbox_dropped_events: self.outbox.dropped_events,
            suppressed_stale_events: self.outbox.suppressed_stale,
            frames_delivered: totals.frames_delivered,
            stale_frames_dropped: totals.stale_frames_dropped,
            demux_discontinuities: totals.demux_discontinuities,
        }
    }

    /// Advance the engine by one poll: drain the command inbox
    /// (applying each through the transition table), then advance
    /// the pipeline by one bounded tick (doc 03 §4.6(b) — commands
    /// apply between ticks). Never blocks.
    pub fn poll(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        // 1. Commands (each receipt precedes the events it caused —
        // doc 05 §4.3 guarantee 2). Draining stops at the first empty
        // OR disconnected recv: both mean "no more work this poll".
        while let Ok(command) = session.inbox.try_recv() {
            let (receipt, outputs) = session.apply_command(command);
            self.outbox.push(EngineOutput::Receipt(receipt));
            for output in outputs {
                self.outbox.push(output);
            }
        }
        // 2. The pipeline.
        let outputs = match session.state {
            SessionState::Opening => session.open_step(),
            SessionState::Buffering | SessionState::Playing | SessionState::Paused => {
                session.tick_step()
            }
            _ => Vec::new(),
        };
        for output in outputs {
            self.outbox.push(output);
        }
        // 3. A closed session is destroyed here — after its outputs
        // (including `state-changed → idle`) are safely in the hub. Its
        // pipeline observability survives at the engine level.
        let totals = session.totals;
        let pair = session.pair();
        let closed = session.closed;
        self.last_pair = Some(pair);
        if closed {
            self.pipeline_totals.merge(&totals);
            self.session = None;
        }
    }

    /// Drain the output stream: receipts always survive;
    /// pipeline-scoped events superseded by a newer generation are
    /// suppressed (doc 05 §4.2 — the engine-side staleness rule).
    #[must_use]
    pub fn take_outputs(&mut self) -> Vec<EngineOutput> {
        let pair = self.current_pair().unwrap_or(GenerationPair {
            epoch: SessionEpoch::FIRST,
            pipeline: PipelineGeneration::FIRST,
        });
        self.outbox.drain(&pair)
    }

    fn current_pair(&self) -> Option<GenerationPair> {
        self.session.as_ref().map(|s| s.pair()).or(self.last_pair)
    }
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl SessionError {
    fn generation_exhausted_boxed() -> SessionError {
        SessionError::GenerationExhausted(EngineError::new(
            media_types::PipelineStage::Control,
            media_types::ErrorKind::ResourceLimit {
                limit: media_types::error::LimitKind::SessionGenerations,
                budget: u64::MAX,
                attempted: u64::MAX,
            },
            media_types::error::ErrorContext::new("generation.exhausted"),
        ))
    }
}

/// The context that travels beside an applied input (the pure table
/// carries no payloads).
#[derive(Default)]
struct InputContext {
    /// The request that caused this application, when attributable
    /// (doc 05 §3.1 `origin`).
    origin: Option<RequestId>,
    /// The typed failure for a `Failed` input.
    fatal: Option<EngineError>,
    /// The resolved seek target for a `Seek` application (the
    /// payload rides the executor's context — the `SeekPipeline`
    /// effect consumes it).
    seek: Option<SeekTarget>,
}

/// The session (single-writer; driven only through `Engine::poll`).
pub(crate) struct Session {
    epoch: SessionEpoch,
    state: SessionState,
    minter: TokenMinter,
    inbox: Receiver<SessionCommand>,
    pipeline: Option<Pipeline>,
    closed: bool,
    opening_attempted: bool,
    tracks: Arc<Vec<media_types::Track>>,
    selected: Vec<bool>,
    seq: Arc<AtomicU64>,
    totals: PipelineTotals,
    runtime: SessionRuntime,
    /// The seek currently executing (doc 03 §6 steps 5–7): recorded
    /// by the `SeekPipeline` effect, completed on the tick path. The
    /// single slot IS the supersession rule — replacing it silently
    /// retires the older generation (doc 05 §4.2).
    inflight_seek: Option<InFlightSeek>,
}

/// The parts of a [`SessionConfig`] the live session retains: the
/// demuxer moved into the pipeline at construction (the pipeline owns
/// it until teardown), the source was consumed at open (access
/// resolution is the demuxer impl's business) — everything else the
/// executor keeps for the session's lifetime.
struct SessionRuntime {
    decoders: Arc<dyn DecoderProvider>,
    clock: Arc<MediaClock>,
    queues: QueuePlan,
    budget: TickBudget,
}

impl Session {
    fn new(
        epoch: SessionEpoch,
        inbox: Receiver<SessionCommand>,
        config: SessionConfig,
        seq: Arc<AtomicU64>,
    ) -> Session {
        let pipeline = Pipeline::new(config.demuxer, Arc::clone(&config.consumer));
        let runtime = SessionRuntime {
            decoders: config.decoders,
            clock: config.clock,
            queues: config.queues,
            budget: config.budget,
        };
        Session {
            epoch,
            state: SessionState::Idle,
            minter: TokenMinter::new(epoch),
            inbox,
            pipeline: Some(pipeline),
            closed: false,
            opening_attempted: false,
            tracks: Arc::new(Vec::new()),
            selected: Vec::new(),
            seq,
            totals: PipelineTotals::default(),
            runtime,
            inflight_seek: None,
        }
    }

    fn pair(&self) -> GenerationPair {
        self.minter.pair()
    }

    fn emit(&self, kind: EngineEventKind, origin: Option<RequestId>) -> EngineOutput {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let pair = self.pair();
        EngineOutput::Event(EngineEvent {
            seq,
            session: pair.session_id(),
            generation: pair.wire_generation(),
            origin,
            kind,
        })
    }

    /// Emit a **seek terminal** under the in-flight seek's own
    /// generation, not the session's current wire generation (doc 05
    /// §4.3 invariant 4: exactly one terminal seek event per completed
    /// seek gen).
    ///
    /// The two can diverge legitimately: a follow-up seek that fails
    /// *resolution* (unresolvable target, unseekable source) still
    /// mints its generation at accept (doc 05 §4.2) but never touches
    /// the pipeline — the older in-flight seek still owns the
    /// pipeline's content and still completes (see `seek.rs`: the
    /// point of no return is the flush). Stamping that terminal with
    /// the newer mint would annex it for a generation that already
    /// has its own `SeekFailed` — two terminals for one gen. The
    /// outbox's drain exemption (doc 05 §4.3) keeps the older-gen
    /// terminal visible across the newer live generation.
    ///
    /// Superseded seeks never reach this path at all: their slot was
    /// replaced before any terminal was emitted, so there is nothing
    /// to stamp (doc 05 §4.2 — superseded seeks are silent).
    fn emit_seek_terminal(
        &self,
        kind: EngineEventKind,
        origin: Option<RequestId>,
        generation: GenerationId,
    ) -> EngineOutput {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let pair = self.pair();
        EngineOutput::Event(EngineEvent {
            seq,
            session: pair.session_id(),
            generation: Generation(generation.counter()),
            origin,
            kind,
        })
    }

    /// Apply one input through the table and interpret its effects.
    /// Answers the outputs **in causal order** (the caller places the
    /// receipt before them for commands). The context travels **by
    /// value** — the `Failed` payload (a typed [`EngineError`], not
    /// `Clone`) is consumed by its effect, never duplicated.
    fn apply(
        &mut self,
        input: SessionInput,
        mut context: InputContext,
    ) -> Result<Vec<EngineOutput>, TransitionRejected> {
        let origin = context.origin;
        let row: &'static Transition = apply_input(self.state, input)?;
        let mut outputs = Vec::new();
        let from = self.state;
        for effect in row.effects {
            match effect {
                Effect::ReportFatal => {
                    let error = context.fatal.take().unwrap_or_else(internal_missing_fatal);
                    outputs.push(self.emit(EngineEventKind::Fatal { error }, origin));
                }
                Effect::DestroySession => {
                    // Doc 03 §5 teardown order, synchronous form:
                    // mint (the row's BumpPipeline ran first), then
                    // interrupt, then flush+close+drop.
                    self.minter.interrupt();
                    let generation = self.pair().pipeline.as_queue_generation();
                    if let Some(pipeline) = self.pipeline.as_mut() {
                        pipeline.teardown(generation);
                    }
                    self.pipeline = None;
                    self.closed = true;
                }
                Effect::BumpPipeline => {
                    // The mint can only fail at u64 exhaustion; the
                    // honest answer is a hard stop (fatal event,
                    // Error state, confirmation event) — the L0
                    // vocabulary's own terminal condition.
                    if let Err(error) = self.minter.mint_pipeline() {
                        outputs.push(self.emit(EngineEventKind::Fatal { error }, origin));
                        let from = self.state;
                        self.state = SessionState::Error;
                        outputs.push(self.emit(
                            EngineEventKind::StateChanged {
                                from,
                                to: SessionState::Error,
                            },
                            origin,
                        ));
                        return Ok(outputs);
                    }
                }
                Effect::TeardownPipeline => {
                    let generation = self.pair().pipeline.as_queue_generation();
                    if let Some(pipeline) = self.pipeline.as_mut() {
                        pipeline.teardown(generation);
                    }
                    self.pipeline = None;
                    // A torn-down pipeline can never complete a seek:
                    // the in-flight slot dies here, silently — the
                    // teardown's mint superseded its generation
                    // (doc 05 §4.2).
                    self.inflight_seek = None;
                }
                Effect::FreezeClock | Effect::StopClock => {
                    // Underrun/pause entry freezes; Ended freezes at
                    // the end position (v0: pause — see the effect's
                    // docs).
                    self.runtime.clock.pause();
                }
                Effect::ResumeClock => {
                    self.runtime.clock.resume();
                }
                Effect::EmitTracks => {
                    outputs.push(self.emit(
                        EngineEventKind::TracksChanged {
                            tracks: Arc::clone(&self.tracks),
                            selected: self.selected.clone(),
                        },
                        origin,
                    ));
                }
                Effect::EmitBufferingEnter => {
                    outputs.push(self.emit(EngineEventKind::Buffering { progress: 0.0 }, origin));
                }
                Effect::EmitBufferingExit => {
                    outputs.push(self.emit(EngineEventKind::Buffering { progress: 1.0 }, origin));
                }
                Effect::EmitEndOfMedia => {
                    outputs.push(self.emit(EngineEventKind::EndOfMedia, origin));
                }
                Effect::SeekPipeline => {
                    self.seek_pipeline_step(context.seek.take(), origin, &mut outputs);
                }
            }
        }
        self.state = row.to;
        // Every applied transition confirms with `state-changed` —
        // self-transitions included (doc 05 §3.2 C8).
        outputs.push(self.emit(EngineEventKind::StateChanged { from, to: row.to }, origin));
        Ok(outputs)
    }

    /// Apply a submitted command: the pure lookup plus the receipt.
    /// The receipt is returned separately so the caller can order it
    /// **before** the events it causes (doc 05 §4.3 guarantee 2).
    fn apply_command(&mut self, command: SessionCommand) -> (CommandReceipt, Vec<EngineOutput>) {
        // A seek without its target payload is a caller bug, surfaced
        // typed at the command boundary (never a panic, never a
        // guessed zero) — the table never sees it.
        if command.kind == SessionCommandKind::Seek && command.seek.is_none() {
            let receipt = CommandReceipt {
                request: command.request,
                command: command.kind,
                result: ReceiptResult::Rejected {
                    state: self.state,
                    reason: RejectionReason::MissingPayload,
                },
            };
            return (receipt, Vec::new());
        }
        let context = InputContext {
            origin: Some(command.request),
            fatal: None,
            seek: command.seek,
        };
        match self.apply(command.kind.as_input(), context) {
            Ok(outputs) => {
                let pair = self.pair();
                let result = if command.kind == SessionCommandKind::Close {
                    // Class S: the close is complete in this receipt
                    // (doc 05 §2.3).
                    ReceiptResult::Completed {
                        session: pair.session_id(),
                    }
                } else {
                    // Class A: accepted under the post-accept
                    // generation (the NEW one for gen-bumping
                    // commands — doc 05 §4.2).
                    ReceiptResult::Accepted {
                        session: pair.session_id(),
                        generation: pair.wire_generation(),
                    }
                };
                let receipt = CommandReceipt {
                    request: command.request,
                    command: command.kind,
                    result,
                };
                (receipt, outputs)
            }
            Err(rejection) => {
                let receipt = CommandReceipt {
                    request: command.request,
                    command: command.kind,
                    result: ReceiptResult::Rejected {
                        state: rejection.from,
                        reason: rejection.reason,
                    },
                };
                (receipt, Vec::new())
            }
        }
    }

    /// Steps 1 (semantic half) through 4 of the seek flow (doc 03
    /// §6) — the `SeekPipeline` effect's interpreter. The row's
    /// `BumpPipeline` has already minted and published the generation
    /// this seek runs under, and `FreezeClock` (the Playing row) has
    /// already frozen the clock at the old position; the accept
    /// receipt still answers class A under the bumped generation
    /// (doc 05 §2.4/§4.2).
    ///
    /// On a typed refusal the pipeline is left exactly as it stood
    /// (contract D4): the queues were not flushed, the demuxer never
    /// moved, and playback continues from the old position. On a
    /// flush or reanchor failure the pipeline re-fills from where the
    /// demuxer still stands (a rebuffer — honest, bounded).
    fn seek_pipeline_step(
        &mut self,
        target: Option<SeekTarget>,
        origin: Option<RequestId>,
        outputs: &mut Vec<EngineOutput>,
    ) {
        let Some(target) = target else {
            // Unreachable through `apply_command` (the payload-less
            // seek is short-circuited to the typed `MissingPayload`
            // rejection there); honored honestly rather than panicked
            // on.
            return;
        };
        let position_ns = clock_position_ns(&self.runtime.clock);
        let duration_ns = self.pipeline.as_ref().and_then(Pipeline::duration_ns);
        // Step 1's semantic half: resolution (clamping is truth —
        // doc 05 §2.4)…
        let resolved_ns = match resolve_target(target, duration_ns, position_ns) {
            ResolvedSeek::Position { ns } => ns,
            ResolvedSeek::Failure(reason) => {
                outputs.push(self.emit(
                    EngineEventKind::SeekFailed {
                        reason,
                        position: position_ns,
                    },
                    origin,
                ));
                return;
            }
        };
        // …and capability: an unseekable source never touches the
        // pipeline at all (doc 03 §6 step 1 — never a fake success,
        // MP §8 bullet 4).
        let seekable = self
            .pipeline
            .as_ref()
            .is_some_and(|pipeline| pipeline.seekability() != SeekSupport::None);
        if !seekable {
            outputs.push(self.emit(
                EngineEventKind::SeekFailed {
                    reason: not_seekable(),
                    position: position_ns,
                },
                origin,
            ));
            return;
        }
        // Steps 3–4 against the live pipeline, under the generation
        // the row just minted.
        let generation = self.pair().pipeline.as_queue_generation();
        let flush_budget = self.runtime.budget.flush_calls_per_limb;
        let outcome = match self.pipeline.as_mut() {
            Some(pipeline) => pipeline_seek(pipeline, generation, resolved_ns, flush_budget),
            // Unreachable: seek-legal states own a live pipeline.
            None => return,
        };
        self.totals.stale_frames_dropped = self
            .totals
            .stale_frames_dropped
            .saturating_add(u64::from(outcome.stale_frames_flushed));
        match outcome.inflight {
            Some(mut inflight) => {
                // The causing request is the terminal's origin (doc 05
                // §3.1). Replacing a previous slot here IS the
                // supersession: the older generation never completes,
                // so it never emits a terminal (doc 05 §4.2).
                inflight.origin = origin;
                self.inflight_seek = Some(inflight);
            }
            None => {
                // D4: the flush happened but the demuxer never moved —
                // the pipeline re-fills from the demuxer's position.
                // The in-flight slot (a superseded older seek
                // included) clears: its generation's pipeline content
                // is gone.
                if let Some(pipeline) = self.pipeline.as_mut() {
                    pipeline.end_seek();
                }
                self.inflight_seek = None;
                if let Some(reason) = outcome.failure {
                    outputs.push(self.emit(
                        EngineEventKind::SeekFailed {
                            reason,
                            position: position_ns,
                        },
                        origin,
                    ));
                }
            }
        }
    }

    /// The `Opening` step (v0's synchronous open/probe): parse the
    /// container, wire the pipeline, raise `OpenReady` — or raise
    /// `Failed` with the typed cause.
    fn open_step(&mut self) -> Vec<EngineOutput> {
        if self.opening_attempted {
            return Vec::new();
        }
        self.opening_attempted = true;
        let started = self
            .pipeline
            .as_mut()
            .map(Pipeline::start_demuxer)
            .unwrap_or_else(|| Err(internal_missing_pipeline()));
        match started {
            Ok(tracks) => {
                let generation = self.pair().pipeline.as_queue_generation();
                let decoders = self.runtime.decoders.as_ref();
                let queues = &self.runtime.queues;
                let wired = self
                    .pipeline
                    .as_mut()
                    .map(|pipeline| pipeline.wire(tracks, decoders, queues, generation));
                match wired {
                    Some(Ok(count)) if count > 0 => {
                        if let Some(pipeline) = self.pipeline.as_ref() {
                            let (tracks, selected) = pipeline.tracks();
                            self.tracks = Arc::new(tracks.to_vec());
                            self.selected = selected.to_vec();
                        }
                        let context = InputContext::default();
                        // The rejection is unreachable (the tests pin
                        // OpenReady legal in Opening); honored honestly.
                        self.apply(SessionInput::OpenReady, context)
                            .unwrap_or_default()
                    }
                    Some(Ok(_)) => {
                        // Nothing decodable: the engine refuses to
                        // fabricate playback (typed config failure).
                        self.fail_with(crate::error::config_error(
                            SessionConfigError::NoDecodableTrack,
                        ))
                    }
                    Some(Err(error)) => self.fail_with(error),
                    None => Vec::new(),
                }
            }
            Err(error) => self.fail_with(error),
        }
    }

    /// One bounded pipeline tick, translating at most one signal into
    /// a transition (priority: fatal, drained, eof, primed, starved —
    /// see [`translate_pipeline_signal`]).
    fn tick_step(&mut self) -> Vec<EngineOutput> {
        let state = self.state;
        let mut report = match self.pipeline.as_mut() {
            Some(pipeline) => pipeline.tick(state, &self.runtime.budget),
            None => return Vec::new(),
        };
        self.totals.merge_tick(&report);
        let mut outputs = Vec::new();
        // Side events first (they are facts about the tick itself).
        if let Some(position) = report.first_frame {
            outputs.push(self.emit(EngineEventKind::FirstFrame { position }, None));
        }
        if report.damaged > 0 {
            outputs.push(self.emit(
                EngineEventKind::DamagedData {
                    count: report.damaged,
                },
                None,
            ));
        }
        // The in-flight seek's completion (doc 03 §6 steps 5–7): when
        // the pre-roll is ready — every enabled limb delivered (or
        // provably never can) — re-anchor the clock on the truth and
        // emit the terminal. This runs before the tick's signal
        // translation so the seek's terminal precedes the state work
        // the same tick causes (causal order: the seek completes,
        // then the session resumes).
        if let Some(inflight) = self.inflight_seek
            && let Some(SeekPreroll::Ready { audio_anchor_ns }) =
                self.pipeline.as_ref().map(Pipeline::seek_preroll)
        {
            let origin = inflight.origin;
            let requested = inflight.target_ns;
            let generation = inflight.generation;
            match complete_seek(&self.runtime.clock, inflight, audio_anchor_ns) {
                Ok(actual) => {
                    outputs.push(self.emit_seek_terminal(
                        EngineEventKind::Seeked { requested, actual },
                        origin,
                        generation,
                    ));
                }
                Err(reason) => outputs.push(self.emit_seek_terminal(
                    EngineEventKind::SeekFailed {
                        reason,
                        position: clock_position_ns(&self.runtime.clock),
                    },
                    origin,
                    generation,
                )),
            }
            // The observation window closes either way: the
            // generation has its exactly-one terminal (doc 05 §4.3
            // invariant 4).
            self.inflight_seek = None;
            if let Some(pipeline) = self.pipeline.as_mut() {
                pipeline.end_seek();
            }
        }
        // One translated input, at most.
        let signal = if report.fatal.is_some() {
            Some(PipelineSignal::Fatal)
        } else if report.drained {
            Some(PipelineSignal::Drained)
        } else if report.eof_reached {
            Some(PipelineSignal::EofReached)
        } else if report.primed {
            Some(PipelineSignal::Primed)
        } else if report.starved {
            Some(PipelineSignal::Starved)
        } else {
            None
        };
        if let Some(signal) = signal
            && let Some(input) = translate_pipeline_signal(state, signal)
        {
            let context = InputContext {
                origin: None,
                fatal: report.fatal.take(),
                seek: None,
            };
            match self.apply(input, context) {
                Ok(mut transition_outputs) => outputs.append(&mut transition_outputs),
                Err(_) => {
                    // Unreachable: the exhaustive tests prove the
                    // translation only yields table-legal inputs.
                    // Honored honestly (dropped) rather than
                    // panicked on.
                }
            }
        }
        outputs
    }

    /// Enter `Error` with a typed fatal cause (the `Failed` input's
    /// executor path for pipeline-open failures).
    fn fail_with(&mut self, error: EngineError) -> Vec<EngineOutput> {
        let context = InputContext {
            origin: None,
            fatal: Some(error),
            seek: None,
        };
        // The rejection is unreachable by construction (the callers
        // only fail in table-legal states); honored honestly.
        self.apply(SessionInput::Failed, context)
            .unwrap_or_default()
    }
}

/// The clock's current position in nanoseconds — the seek target's
/// resolution base and the failed-seek report's rollback position.
/// The clock answers in the nanosecond base, so the `Err` arm is the
/// i64-exhaustion corner only; 0 is the honest floor there, never a
/// guess presented as truth.
fn clock_position_ns(clock: &MediaClock) -> i64 {
    match clock.now().as_nanos() {
        Ok(ns) => nanos_i64(ns),
        Err(_) => 0,
    }
}

/// The typed stand-in for a `Failed` application that arrived
/// without a payload (a caller bug — surfaced, never panicked on).
fn internal_missing_fatal() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Control,
        media_types::ErrorKind::ResourceLimit {
            limit: media_types::error::LimitKind::Memory,
            budget: 0,
            attempted: 0,
        },
        media_types::error::ErrorContext::new("engine.fatal-missing"),
    )
}

/// The typed stand-in for a tick with no pipeline (same class of
/// impossible-by-construction breach).
fn internal_missing_pipeline() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Control,
        media_types::ErrorKind::ResourceLimit {
            limit: media_types::error::LimitKind::Memory,
            budget: 0,
            attempted: 0,
        },
        media_types::error::ErrorContext::new("engine.pipeline-missing"),
    )
}

/// The whole transition table is reachable from the pure model —
/// exposed for the exhaustive tests (and for future model checkers
/// per doc 03 §4.6(f)).
#[must_use]
pub fn transition_table() -> &'static [Transition] {
    TRANSITIONS
}
