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
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};

use commands::CommandId;
use commands::envelope::{Generation, RequestId, SessionId};
use media_clock::{MediaClock, PlaybackClock};
use media_types::{EngineError, GenerationId, SourceUri};
use plugin_api::SeekSupport;

use crate::epoch::{CancelToken, GenerationPair, PipelineGeneration, SessionEpoch, TokenMinter};
use crate::error::{SessionConfigError, SessionError};
use crate::event::{EngineEvent, EngineEventKind, media_info_wire};
use crate::events::{DamageCoalescer, EventPolicy, ProgressWindow, TimeTickPolicy};
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

/// The outbox capacity (live events; receipts are bounded separately
/// by [`RECEIPT_CAPACITY`] and are never dropped for *event*
/// overflow). `[PROPOSAL: doc 05 §3.4's burst bound is 1000 events/s
/// to a host; the outbox holds one drain interval's worth — hosts
/// that never drain see oldest events dropped and counted rather
/// than unbounded memory]`.
pub const OUTBOX_CAPACITY: usize = 1024;

/// The hard cap on undelivered command receipts (engineering
/// directive D-1 boundedness — the 2026-10-22 directives-audit M1
/// fix): a host that never drains loses its oldest undelivered
/// receipt (counted in [`EngineStats::outbox_dropped_receipts`])
/// before the hub can grow without bound. `[PROPOSAL: the
/// wire-visible surfacing of a dropped receipt lands with
/// `system.log` (the burst-bound drop-refinement row); the typed
/// counter is the honest observable today — a host that misses a
/// receipt re-queries or times out, the truthful-UI timeout path]`.
pub const RECEIPT_CAPACITY: usize = 4096;

/// The command inbox bound (engineering directive D-1 boundedness —
/// directives-audit M1): submissions beyond it are refused typed at
/// submit time ([`SessionError::InboxBusy`]) — backpressure by
/// rejection, never unbounded growth. The engine drains the whole
/// inbox every [`Engine::poll`], so the bound only bites a host
/// that submits two orders of magnitude faster than it polls; the
/// command vocabulary has no busy/rejected wire row (frozen
/// catalog, D-017) and the refusal is engine-transport-side — the
/// same family as [`SessionError::SessionClosed`], which hosts
/// already surface as a typed submit failure.
pub const INBOX_CAPACITY: usize = 64;

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
/// through [`Engine::take_outputs`] (or, allocation-free,
/// [`Engine::drain_outputs_into`]).
///
/// The handle is cheap to clone and `Send`; submitting after the
/// session closed answers the typed [`SessionError::SessionClosed`]
/// — the command-side staleness rejection (a handle outliving its
/// session's epoch) — and submitting into a full inbox answers the
/// typed [`SessionError::InboxBusy`] (the bounded-inbox
/// backpressure, doc 03 §4.6(c)'s non-blocking discipline).
#[derive(Debug, Clone)]
pub struct SessionHandle {
    epoch: SessionEpoch,
    tx: SyncSender<SessionCommand>,
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
    /// gone (stale handle) — never a silent drop;
    /// [`SessionError::InboxBusy`] when the bounded inbox is full
    /// (the command was NOT queued — retry or shed load; never a
    /// block, never a drop).
    pub fn submit(&self, request: RequestId, kind: SessionCommandKind) -> Result<(), SessionError> {
        self.queue(SessionCommand {
            request,
            kind,
            seek: None,
        })
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
        self.queue(SessionCommand {
            request,
            kind: SessionCommandKind::Seek,
            seek: Some(target),
        })
    }

    /// The single enqueue path: never blocks (a bounded inbox is a
    /// typed busy, not a stall — doc 03 §4.6(c)/§9), never drops.
    fn queue(&self, command: SessionCommand) -> Result<(), SessionError> {
        self.tx.try_send(command).map_err(|failure| match failure {
            TrySendError::Full(_) => SessionError::InboxBusy,
            TrySendError::Disconnected(_) => SessionError::SessionClosed,
        })
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
    /// The event-emission policy (card 3.B4, doc 05 §3.4): the
    /// `time-tick` interval and the injected monotonic source every
    /// throttle window reads. Defaults to the doc-default interval
    /// over real monotonic time; deterministic hosts inject a
    /// virtual source.
    pub throttle: EventPolicy,
}

impl SessionConfig {
    /// A config with the given stage objects and doc-default queue
    /// plan, tick budget and event policy.
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
            throttle: EventPolicy::default_policy(),
        }
    }
}

/// One slot in the output hub: live, or a tombstone left by a drop
/// or a tick replacement. Tombstones keep every remaining index
/// **stable between drains** (pushes append, drops overwrite in
/// place — nothing shifts), which is what lets the drop cursors
/// below stay O(1) to maintain; they are reclaimed in batches by
/// [`OutputHub::compact_if_due`] so the hub's memory stays bounded
/// at roughly twice the live population.
#[derive(Debug)]
enum Slot {
    /// A live output, awaiting delivery.
    Live(EngineOutput),
    /// A dropped or replaced output — already counted at its drop
    /// site; skipped at drain, reclaimed at compaction.
    Dead,
}

/// The engine's output buffer with its staleness, overflow and
/// tick-replacement policies (see the module docs).
///
/// # Complexity (engineering directive D-2; fixes the
/// 2026-10-22 directives-audit's O(n)-push finding)
///
/// [`OutputHub::push`] is **O(1) amortized with no memmoves**:
/// capacity drops and tick replacements tombstone their slot
/// instead of scanning-and-shifting the queue; the oldest-event,
/// oldest-receipt and resident-tick cursors only ever move forward
/// (each index is scanned past at most once between drains, so the
/// scans amortize into the pushes that created the items); and
/// compaction — an in-place `VecDeque::retain`, zero allocation —
/// runs only once the dead population rivals the live one, its
/// O(live) cost amortizing to O(1) per push while total slots stay
/// ≤ ~2× the live bound.
#[derive(Debug)]
struct OutputHub {
    /// Live and dead slots in delivery (causal) order.
    queue: VecDeque<Slot>,
    /// The live-event bound (doc 05 §3.4's burst-bound mechanism).
    capacity: usize,
    /// The live-receipt bound (D-1 boundedness).
    receipts_capacity: usize,
    /// Live `Event` items currently resident.
    live_events: usize,
    /// Live `Receipt` items currently resident.
    live_receipts: usize,
    /// Index of the oldest live event — the capacity-drop victim.
    /// `None` iff `live_events == 0`.
    first_event: Option<usize>,
    /// Index of the oldest live receipt — the receipt-cap victim.
    /// `None` iff `live_receipts == 0`.
    first_receipt: Option<usize>,
    /// Index of the resident (undelivered) `timing.time-tick`, if
    /// any — doc 05 §3.4 rule 1's "at most one tick in flight".
    resident_tick: Option<usize>,
    dropped_events: u64,
    dropped_receipts: u64,
    time_ticks_replaced: u64,
    suppressed_stale: u64,
}

impl OutputHub {
    fn new(capacity: usize, receipts_capacity: usize) -> OutputHub {
        OutputHub {
            queue: VecDeque::new(),
            capacity: capacity.max(1),
            receipts_capacity: receipts_capacity.max(1),
            live_events: 0,
            live_receipts: 0,
            first_event: None,
            first_receipt: None,
            resident_tick: None,
            dropped_events: 0,
            dropped_receipts: 0,
            time_ticks_replaced: 0,
            suppressed_stale: 0,
        }
    }

    /// Append one output, applying the overflow and tick-replacement
    /// policies in place (see the struct docs for the complexity
    /// contract). At event capacity the **oldest live event** is
    /// tombstoned and counted — receipts are pinned from *event*
    /// overflow (doc 05 §4.3: responses are never suppressed by
    /// events) and bounded separately by their own hard cap. A newer
    /// `timing.time-tick` replaces an undelivered older one (counted)
    /// instead of consuming capacity — doc 05 §3.4 rule 1.
    fn push(&mut self, output: EngineOutput) {
        match &output {
            EngineOutput::Receipt(_) => {
                if self.live_receipts >= self.receipts_capacity {
                    self.drop_oldest_receipt();
                }
                let index = self.queue.len();
                self.queue.push_back(Slot::Live(output));
                self.live_receipts += 1;
                if self.first_receipt.is_none() {
                    self.first_receipt = Some(index);
                }
            }
            EngineOutput::Event(event) => {
                let is_tick = matches!(event.kind, EngineEventKind::TimeTick { .. });
                if is_tick && let Some(index) = self.resident_tick {
                    // Never queued: the fresher position supersedes
                    // the stale one. The replaced tick never reaches
                    // the wire, so the host's stream stays ordered;
                    // the seq gap it leaves is the class doc 05
                    // §4.3.1 already anticipates ("gaps indicate
                    // filtered or dropped events — never reordering").
                    self.kill(index);
                    self.live_events -= 1;
                    self.time_ticks_replaced = self.time_ticks_replaced.saturating_add(1);
                    if self.first_event == Some(index) {
                        self.first_event = self.next_event_after(index + 1);
                    }
                }
                if self.live_events >= self.capacity {
                    self.drop_oldest_event();
                }
                let index = self.queue.len();
                self.queue.push_back(Slot::Live(output));
                self.live_events += 1;
                if self.first_event.is_none() {
                    self.first_event = Some(index);
                }
                if is_tick {
                    self.resident_tick = Some(index);
                }
            }
        }
        self.compact_if_due();
    }

    /// Tombstone the oldest live event and advance the cursor.
    fn drop_oldest_event(&mut self) {
        if let Some(index) = self.first_event {
            if self.resident_tick == Some(index) {
                self.resident_tick = None;
            }
            self.kill(index);
            self.live_events -= 1;
            self.dropped_events = self.dropped_events.saturating_add(1);
            self.first_event = self.next_event_after(index + 1);
        }
    }

    /// Tombstone the oldest live receipt and advance the cursor.
    fn drop_oldest_receipt(&mut self) {
        if let Some(index) = self.first_receipt {
            self.kill(index);
            self.live_receipts -= 1;
            self.dropped_receipts = self.dropped_receipts.saturating_add(1);
            self.first_receipt = self.next_receipt_after(index + 1);
        }
    }

    fn kill(&mut self, index: usize) {
        self.queue[index] = Slot::Dead;
    }

    /// The next live event at or after `from` (indices are stable
    /// between drains — pushes and tombstones never shift them).
    fn next_event_after(&self, from: usize) -> Option<usize> {
        self.queue
            .iter()
            .skip(from)
            .position(|slot| matches!(slot, Slot::Live(EngineOutput::Event(_))))
            .map(|offset| from + offset)
    }

    /// The next live receipt at or after `from`.
    fn next_receipt_after(&self, from: usize) -> Option<usize> {
        self.queue
            .iter()
            .skip(from)
            .position(|slot| matches!(slot, Slot::Live(EngineOutput::Receipt(_))))
            .map(|offset| from + offset)
    }

    /// Reclaim tombstones once they rival the live population: an
    /// in-place `retain` (no allocation) plus one cursor-recompute
    /// pass, both O(live + dead) — amortized O(1) per push under the
    /// trigger's accounting, and the reason the hub's memory stays
    /// bounded at ~2× the live bound however long the host stalls.
    fn compact_if_due(&mut self) {
        let live = self.live_events + self.live_receipts;
        let dead = self.queue.len() - live;
        if dead == 0 || dead < live {
            return;
        }
        self.queue.retain(|slot| !matches!(slot, Slot::Dead));
        self.first_event = None;
        self.first_receipt = None;
        self.resident_tick = None;
        for (index, slot) in self.queue.iter().enumerate() {
            match slot {
                Slot::Live(EngineOutput::Receipt(_)) => {
                    if self.first_receipt.is_none() {
                        self.first_receipt = Some(index);
                    }
                }
                Slot::Live(EngineOutput::Event(event)) => {
                    if self.first_event.is_none() {
                        self.first_event = Some(index);
                    }
                    if self.resident_tick.is_none()
                        && matches!(event.kind, EngineEventKind::TimeTick { .. })
                    {
                        self.resident_tick = Some(index);
                    }
                }
                Slot::Dead => {}
            }
        }
    }

    /// Drain into a caller-owned buffer (the reuse API — engineering
    /// directive D-1: no per-drain allocation; the host keeps one
    /// `Vec` alive across polls), suppressing pipeline-scoped events
    /// emitted under generations older than `current` (doc 05 §4.2 —
    /// the engine-side staleness rule: superseded work is silent) —
    /// with one exemption: the two seek terminals. A `Seeked`/
    /// `SeekFailed` already emitted for a completed generation
    /// survives a later mint that lands before the host drains (doc
    /// 05 §4.3 invariant 4: exactly one terminal per completed
    /// generation). Superseded seeks stay silent the honest way —
    /// their slot was replaced before any terminal was emitted, so
    /// there is nothing in the buffer to swallow.
    fn drain_into(&mut self, current: &GenerationPair, out: &mut Vec<EngineOutput>) {
        let live = current.pipeline.0;
        while let Some(slot) = self.queue.pop_front() {
            match slot {
                Slot::Dead => {}
                Slot::Live(EngineOutput::Receipt(receipt)) => {
                    out.push(EngineOutput::Receipt(receipt));
                }
                Slot::Live(EngineOutput::Event(event)) => {
                    let seek_terminal = matches!(
                        event.kind,
                        EngineEventKind::Seeked { .. } | EngineEventKind::SeekFailed { .. }
                    );
                    if event.generation.0 < live && !seek_terminal {
                        self.suppressed_stale = self.suppressed_stale.saturating_add(1);
                    } else {
                        out.push(EngineOutput::Event(event));
                    }
                }
            }
        }
        self.live_events = 0;
        self.live_receipts = 0;
        self.first_event = None;
        self.first_receipt = None;
        self.resident_tick = None;
    }
}

/// Observability counters for the output path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EngineStats {
    /// Events dropped for outbox capacity (oldest-first; receipts
    /// are never dropped for event overflow).
    pub outbox_dropped_events: u64,
    /// Receipts dropped at the receipt hard cap (D-1 boundedness;
    /// zero unless the host leaves > [`RECEIPT_CAPACITY`] commands'
    /// responses undelivered — a host bug, surfaced here).
    pub outbox_dropped_receipts: u64,
    /// Undelivered time-ticks replaced by a newer one (doc 05 §3.4
    /// rule 1 — the "never queued" replacement, counted).
    pub time_ticks_replaced: u64,
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

    /// An engine with an explicit outbox capacity (tests). The
    /// receipt bound stays at the production [`RECEIPT_CAPACITY`].
    #[must_use]
    pub fn with_outbox_capacity(capacity: usize) -> Engine {
        Engine {
            next_epoch: SessionEpoch::FIRST.0,
            seq: Arc::new(AtomicU64::new(0)),
            session: None,
            outbox: OutputHub::new(capacity, RECEIPT_CAPACITY),
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
        let (tx, rx) = std::sync::mpsc::sync_channel(INBOX_CAPACITY);
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
            outbox_dropped_receipts: self.outbox.dropped_receipts,
            time_ticks_replaced: self.outbox.time_ticks_replaced,
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

    /// Drain the output stream into a caller-owned, reused buffer
    /// (engineering directive D-1 — the host-side pattern is
    /// `out.clear()` + `engine.drain_outputs_into(&mut out)` per
    /// poll, so steady state allocates nothing): receipts always
    /// survive; pipeline-scoped events superseded by a newer
    /// generation are suppressed (doc 05 §4.2 — the engine-side
    /// staleness rule).
    pub fn drain_outputs_into(&mut self, out: &mut Vec<EngineOutput>) {
        let pair = self.current_pair().unwrap_or(GenerationPair {
            epoch: SessionEpoch::FIRST,
            pipeline: PipelineGeneration::FIRST,
        });
        self.outbox.drain_into(&pair, out);
    }

    /// Drain the output stream into a fresh `Vec` — the owned form
    /// of [`Engine::drain_outputs_into`] for callers that prefer it;
    /// the contents and ordering are identical.
    #[must_use]
    pub fn take_outputs(&mut self) -> Vec<EngineOutput> {
        let mut out = Vec::new();
        self.drain_outputs_into(&mut out);
        out
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
    /// The source the session was opened from — the `media.info-ready`
    /// snapshot's origin (card 3.B4; the URI's redacted display form is
    /// the wire-safe rendering, V8).
    source: SourceUri,
    /// The card 3.B4 emission state (doc 05 §3.2/§3.4): everything the
    /// throttle windows remember between ticks.
    emissions: EmissionState,
    /// The seek currently executing (doc 03 §6 steps 5–7): recorded
    /// by the `SeekPipeline` effect, completed on the tick path. The
    /// single slot IS the supersession rule — replacing it silently
    /// retires the older generation (doc 05 §4.2).
    inflight_seek: Option<InFlightSeek>,
}

/// The per-session emission-policy state (card 3.B4): the baselines
/// and windows the doc 05 §3.4 rules need across ticks. All plain
/// data — the steady-state emission paths allocate nothing (the
/// once-per-open `MediaInfo` snapshot and the ≤ 1 Hz non-fatal
/// error flush are the documented low-rate exceptions, doc 05 §3.4
/// rule 3: "bounded naturally by the operations that cause them").
struct EmissionState {
    /// The last emitted tick's position (ns) — `None` until the first
    /// Playing tick sets the baseline (doc 05 §3.2: no immediate tick
    /// on play; the immediate cases are pause entry and seek
    /// completion).
    last_tick_ns: Option<i64>,
    /// The in-episode buffering window and the last emitted fraction
    /// (doc 05 §3.4 rule 2: monotonic within one episode, ≤ 5 Hz;
    /// reset at every episode's `0.0` bookend).
    buffering_window: ProgressWindow,
    buffering_last: f32,
    /// The non-fatal damage coalescer (doc 05 §3.2: 1 s window,
    /// flushed at terminals so the last count is never lost).
    damage: DamageCoalescer,
    /// Whether `duration-known` has fired for this open (once per
    /// open, doc 05 §3.2).
    duration_emitted: bool,
    /// Whether `media.info-ready` has fired for this open.
    info_emitted: bool,
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
    /// The event-emission policy (card 3.B4, doc 05 §3.4): the tick
    /// interval and the injected monotonic source the throttle
    /// windows read.
    tick: TimeTickPolicy,
    throttle: Arc<dyn media_clock::MonotonicSource>,
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
            tick: config.throttle.tick,
            throttle: Arc::clone(&config.throttle.throttle),
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
            source: config.source,
            emissions: EmissionState {
                last_tick_ns: None,
                buffering_window: ProgressWindow::for_buffering(),
                buffering_last: 0.0,
                damage: DamageCoalescer::new(),
                duration_emitted: false,
                info_emitted: false,
            },
            inflight_seek: None,
        }
    }

    fn pair(&self) -> GenerationPair {
        self.minter.pair()
    }

    fn emit(&self, kind: EngineEventKind, origin: Option<RequestId>) -> EngineOutput {
        Self::emit_raw(&self.seq, self.pair(), kind, origin)
    }

    /// The emission core, free of `&self` — the borrow-splitting form
    /// the end-of-track path needs (it iterates the pipeline's reused
    /// scratch while emitting; field-disjoint borrows only).
    fn emit_raw(
        seq: &AtomicU64,
        pair: GenerationPair,
        kind: EngineEventKind,
        origin: Option<RequestId>,
    ) -> EngineOutput {
        let seq = seq.fetch_add(1, Ordering::Relaxed) + 1;
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
        // Doc 05 §3.2: an immediate `time-tick` on **pause entry** —
        // the position the session freezes at, emitted before the
        // `state-changed → paused` so the tick is the last fact of
        // the playing era. Self-transitions (C8 pause-while-paused)
        // are confirmations, not entries — no tick.
        if input == SessionInput::Pause
            && from == SessionState::Playing
            && row.to == SessionState::Paused
        {
            let position = clock_position_ns(&self.runtime.clock);
            self.emissions.last_tick_ns = Some(position);
            outputs.push(self.emit(
                EngineEventKind::TimeTick {
                    position_ns: position,
                    fraction: self.position_fraction(position),
                },
                origin,
            ));
        }
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
                    // A fresh episode resets the in-episode policy
                    // state (doc 05 §3.4 rule 2: monotonic *within one
                    // episode*, windowed per episode).
                    self.emissions.buffering_window = ProgressWindow::for_buffering();
                    self.emissions.buffering_last = 0.0;
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
        // Doc 05 §3.2 (card 3.B4): a non-fatal damage count pending
        // when playback settles is flushed BEFORE the settling
        // `state-changed` — the damage is a fact about the playback
        // that just ended, never lost to the transition (the 1 s
        // window is a rate rule, not a loss rule).
        if !matches!(
            row.to,
            SessionState::Opening
                | SessionState::Buffering
                | SessionState::Playing
                | SessionState::Paused
        ) {
            self.flush_damage(&mut outputs, origin);
        }
        self.state = row.to;
        // Every applied transition confirms with `state-changed` —
        // self-transitions included (doc 05 §3.2 C8).
        outputs.push(self.emit(EngineEventKind::StateChanged { from, to: row.to }, origin));
        Ok(outputs)
    }

    /// Flush the non-fatal damage coalescer unconditionally
    /// (terminal points — ended, error, stopped, idle/teardown — and
    /// the window's due check lives on the tick path).
    fn flush_damage(&mut self, outputs: &mut Vec<EngineOutput>, origin: Option<RequestId>) {
        if let Some((demux, decode)) = self.emissions.damage.flush_all() {
            if demux > 0 {
                outputs.push(self.emit(
                    EngineEventKind::RecoveredDamage {
                        stage: media_types::PipelineStage::Demux,
                        count: demux,
                    },
                    origin,
                ));
            }
            if decode > 0 {
                outputs.push(self.emit(
                    EngineEventKind::RecoveredDamage {
                        stage: media_types::PipelineStage::Decode,
                        count: decode,
                    },
                    origin,
                ));
            }
        }
    }

    /// The tick's `time-tick` fraction: the position as a clamped
    /// share of the known duration (`0.0` when unknown — the honest
    /// floor, never a guess).
    fn position_fraction(&self, position_ns: i64) -> f32 {
        match self.pipeline.as_ref().and_then(Pipeline::duration_ns) {
            Some(duration_ns) if duration_ns > 0 => {
                (position_ns as f64 / duration_ns as f64).clamp(0.0, 1.0) as f32
            }
            _ => 0.0,
        }
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
                        let mut outputs = self
                            .apply(SessionInput::OpenReady, context)
                            .unwrap_or_default();
                        // The card 3.B4 open facts, once per open:
                        // `duration-known` and `media.info-ready`,
                        // after the track list (the snapshot includes
                        // it) and before any first `→ playing` (doc 05
                        // §3.2/§4.3.4 — the only ordering pins).
                        self.emit_open_facts(&mut outputs);
                        outputs
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
    /// see [`translate_pipeline_signal`]) — and running the card
    /// 3.B4 emission policies (doc 05 §3.4) over the tick's raw
    /// facts: the continuous `time-tick` while Playing, the
    /// in-episode `buffering` progress, the `end-of-track`
    /// candidates, and the non-fatal damage windows.
    fn tick_step(&mut self) -> Vec<EngineOutput> {
        let state = self.state;
        let mut report = match self.pipeline.as_mut() {
            Some(pipeline) => pipeline.tick(state, &self.runtime.budget),
            None => return Vec::new(),
        };
        self.totals.merge_tick(&report);
        let now = self.runtime.throttle.now_ns();
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
            // The wire-visible half is coalesced (doc 05 §3.2: 1 s
            // window with a count, per stage).
            self.emissions
                .damage
                .record(report.damaged_demux, report.damaged_decode, now);
        }
        // `playback.end-of-track` (card 3.B4): limbs newly drained
        // this tick — emitted only while the media continues (the
        // tick that fully drains the pipeline ends the media, and
        // `end-of-media` speaks for it; the last limb's end IS the
        // media's end). The pairs ride the pipeline's reused scratch
        // (zero steady-state allocation, D-1) — the borrow-split
        // `emit_raw` form keeps the drain loop allocation-free too.
        let emit_ends = !report.drained;
        {
            let seq = Arc::clone(&self.seq);
            let pair = self.pair();
            if let Some(pipeline) = self.pipeline.as_mut() {
                for (track_kind, id) in pipeline.take_ended_limbs() {
                    if emit_ends {
                        outputs.push(Self::emit_raw(
                            &seq,
                            pair,
                            EngineEventKind::EndOfTrack { track_kind, id },
                            None,
                        ));
                    }
                }
            }
        }
        // The in-episode `session.buffering` progress (card 3.B4,
        // doc 05 §3.4 rule 2): monotonic within the episode, ≤ 5 Hz.
        // The tick that ends the episode (primed / short-media eof)
        // skips its in-episode sample — the `1.0` exit bookend that
        // same tick carries the same information without a duplicate.
        if state == SessionState::Buffering && !(report.primed || report.eof_reached) {
            let fill = report.buffer_fill;
            if fill > self.emissions.buffering_last && self.emissions.buffering_window.permits(now)
            {
                self.emissions.buffering_last = fill;
                outputs.push(self.emit(EngineEventKind::Buffering { progress: fill }, None));
            }
        }
        // The non-fatal damage window's due flush (the terminal
        // flush lives in `apply`'s settling-state hook).
        if let Some((demux, decode)) = self.emissions.damage.flush_if_due(now) {
            if demux > 0 {
                outputs.push(self.emit(
                    EngineEventKind::RecoveredDamage {
                        stage: media_types::PipelineStage::Demux,
                        count: demux,
                    },
                    None,
                ));
            }
            if decode > 0 {
                outputs.push(self.emit(
                    EngineEventKind::RecoveredDamage {
                        stage: media_types::PipelineStage::Decode,
                        count: decode,
                    },
                    None,
                ));
            }
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
                    // Doc 05 §3.2: an immediate `time-tick` after
                    // `seek-completed` — the landing truth, right
                    // where the clock re-anchored (the last fact of
                    // the seeking episode).
                    let position = clock_position_ns(&self.runtime.clock);
                    self.emissions.last_tick_ns = Some(position);
                    outputs.push(self.emit(
                        EngineEventKind::TimeTick {
                            position_ns: position,
                            fraction: self.position_fraction(position),
                        },
                        origin,
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
        // The continuous `timing.time-tick` (card 3.B4, doc 05
        // §3.2/§3.4 rule 1): judged on the media position — the
        // first Playing tick only sets the baseline (no immediate
        // tick on play), then one tick per interval of advanced
        // media time. A frozen test clock therefore emits none,
        // deterministically.
        if self.state == SessionState::Playing {
            let position = clock_position_ns(&self.runtime.clock);
            match self.emissions.last_tick_ns {
                None => self.emissions.last_tick_ns = Some(position),
                Some(last) if self.runtime.tick.is_due(Some(last), position) => {
                    self.emissions.last_tick_ns = Some(position);
                    outputs.push(self.emit(
                        EngineEventKind::TimeTick {
                            position_ns: position,
                            fraction: self.position_fraction(position),
                        },
                        None,
                    ));
                }
                _ => {}
            }
        }
        outputs
    }

    /// The once-per-open facts (card 3.B4): `timing.duration-known`
    /// when the greatest declared track extent is derivable, and
    /// `media.info-ready` always — both before any first
    /// `state-changed → playing` (doc 05 §3.2/§4.3.4).
    fn emit_open_facts(&mut self, outputs: &mut Vec<EngineOutput>) {
        let duration_ns = self.pipeline.as_ref().and_then(Pipeline::duration_ns);
        if !self.emissions.duration_emitted
            && let Some(duration_ns) = duration_ns
        {
            self.emissions.duration_emitted = true;
            outputs.push(self.emit(EngineEventKind::DurationKnown { duration_ns }, None));
        }
        if !self.emissions.info_emitted {
            self.emissions.info_emitted = true;
            let seekable = self
                .pipeline
                .as_ref()
                .is_some_and(|pipeline| pipeline.seekability() != SeekSupport::None);
            let info = Box::new(media_info_wire(
                &self.source,
                duration_ns,
                seekable,
                &self.tracks,
                &self.selected,
            ));
            outputs.push(self.emit(EngineEventKind::MediaInfoReady { info }, None));
        }
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

#[cfg(test)]
mod tests {
    //! The output-hub policy unit tests (card 3.B4 + the addendum's
    //! boundedness/O(1) findings): these live here, not in the
    //! integration suite, because they pin the hub's *internal*
    //! complexity contract — the observable smoke (drop-oldest at
    //! capacity, receipts pinned) is already pinned session-level in
    //! `tests/session_flow.rs`.

    use super::*;
    use commands::envelope::RequestId;
    use testkit::Must;

    fn pair_at(pipeline: u64) -> GenerationPair {
        GenerationPair {
            epoch: SessionEpoch(1),
            pipeline: PipelineGeneration(pipeline),
        }
    }

    fn marker_event(seq: u64) -> EngineOutput {
        EngineOutput::Event(EngineEvent {
            seq,
            session: SessionId(1),
            generation: Generation(0),
            origin: None,
            kind: EngineEventKind::EndOfMedia,
        })
    }

    fn tick_event(seq: u64, position_ns: i64) -> EngineOutput {
        EngineOutput::Event(EngineEvent {
            seq,
            session: SessionId(1),
            generation: Generation(0),
            origin: None,
            kind: EngineEventKind::TimeTick {
                position_ns,
                fraction: 0.0,
            },
        })
    }

    fn receipt(request: u64) -> EngineOutput {
        EngineOutput::Receipt(CommandReceipt {
            request: RequestId(request),
            command: SessionCommandKind::Play,
            result: ReceiptResult::Completed {
                session: SessionId(1),
            },
        })
    }

    /// Push 10k events into a capped hub without draining: the live
    /// population stays at the bound, every overflow is counted, and
    /// — the O(1)-shape pin — the SLOT count stays bounded (the
    /// compaction reclaims tombstones; the pre-3.B4 scan-and-memmove
    /// implementation would also have left the len bounded, but at a
    /// per-push O(n) cost this test's 10k pushes make unmissable in
    /// a profiler, and the drop-count arithmetic below is the
    /// behavioral half of the same pin).
    #[test]
    fn the_hub_stays_bounded_and_counts_every_drop_at_capacity() {
        let mut hub = OutputHub::new(8, 16);
        for seq in 0..10_000u64 {
            hub.push(marker_event(seq));
        }
        assert_eq!(hub.live_events, 8, "the live bound holds");
        assert_eq!(hub.dropped_events, 10_000 - 8, "every overflow counted");
        // The compaction bound: slots <= 2x the live population (+1
        // in-flight) — however long the host stalls.
        let live = hub.live_events + hub.live_receipts;
        assert!(
            hub.queue.len() <= 2 * live + 1,
            "slot count {} not bounded by 2x live {}",
            hub.queue.len(),
            live
        );
        // The survivors are the NEWEST events, in order.
        let mut drained = Vec::new();
        hub.drain_into(&pair_at(0), &mut drained);
        let seqs: Vec<u64> = drained
            .iter()
            .filter_map(|output| match output {
                EngineOutput::Event(event) => Some(event.seq),
                _ => None,
            })
            .collect();
        assert_eq!(seqs, (10_000 - 8..10_000).collect::<Vec<_>>());
    }

    /// Doc 05 §3.4 rule 1: never queued — a newer tick replaces the
    /// undelivered older one (counted), the delivery order keeps the
    /// interleaved non-tick events, and a drained tick frees the
    /// resident slot (the next tick is resident, not a replacement).
    #[test]
    fn a_newer_tick_replaces_the_undelivered_older_one() {
        let mut hub = OutputHub::new(8, 16);
        hub.push(tick_event(1, 250_000_000));
        hub.push(marker_event(2));
        hub.push(tick_event(3, 500_000_000));
        assert_eq!(hub.time_ticks_replaced, 1, "the stale tick replaced");
        assert_eq!(hub.live_events, 2, "marker + one live tick");
        let mut drained = Vec::new();
        hub.drain_into(&pair_at(0), &mut drained);
        let seqs: Vec<u64> = drained
            .iter()
            .filter_map(|output| match output {
                EngineOutput::Event(event) => Some(event.seq),
                _ => None,
            })
            .collect();
        assert_eq!(seqs, vec![2, 3], "the newer tick rides in stream order");
        // After a drain there is no resident tick: the next tick is
        // fresh, not a replacement.
        hub.push(tick_event(4, 750_000_000));
        assert_eq!(hub.time_ticks_replaced, 1);
    }

    /// The resident tick can also be the oldest event — capacity
    /// pressure may drop it like any other (never queued means never
    /// pinned), and the replacement counter stays honest.
    #[test]
    fn capacity_pressure_may_drop_the_resident_tick_and_counts_it() {
        let mut hub = OutputHub::new(2, 16);
        hub.push(tick_event(1, 0));
        hub.push(marker_event(2));
        hub.push(marker_event(3));
        assert_eq!(hub.dropped_events, 1, "the oldest event (the tick) dropped");
        assert!(hub.resident_tick.is_none(), "the cursor died with it");
        hub.push(tick_event(4, 0));
        assert_eq!(hub.time_ticks_replaced, 0, "nothing to replace");
        assert_eq!(hub.live_events, 2);
    }

    /// Receipts are pinned from event overflow but hard-capped
    /// themselves (D-1): beyond the cap the OLDEST undelivered
    /// receipt is dropped and counted — the host loses responses
    /// before the engine loses boundedness.
    #[test]
    fn receipts_are_pinned_from_events_but_hard_capped() {
        let mut hub = OutputHub::new(4, 4);
        for request in 0..4u64 {
            hub.push(receipt(request));
        }
        // Event overflow pressure: receipts survive.
        for seq in 0..20u64 {
            hub.push(marker_event(seq));
        }
        assert_eq!(hub.dropped_events, 16);
        assert_eq!(hub.live_receipts, 4, "pinned from event overflow");
        // Receipt overflow: oldest receipt goes, counted.
        hub.push(receipt(100));
        assert_eq!(hub.dropped_receipts, 1);
        assert_eq!(hub.live_receipts, 4);
        let mut drained = Vec::new();
        hub.drain_into(&pair_at(0), &mut drained);
        let receipts: Vec<u64> = drained
            .iter()
            .filter_map(|output| match output {
                EngineOutput::Receipt(receipt) => Some(receipt.request.0),
                _ => None,
            })
            .collect();
        assert_eq!(receipts, vec![1, 2, 3, 100], "the oldest receipt dropped");
        // And the receipts precede the events they were interleaved
        // with (delivery order preserved).
        let kinds: Vec<bool> = drained
            .iter()
            .map(|o| matches!(o, EngineOutput::Receipt(_)))
            .collect();
        assert_eq!(kinds.first(), Some(&true));
    }

    /// The engine-level view of the bounded inbox (D-1): submissions
    /// beyond the bound answer the typed busy refusal — never a
    /// block, never a drop — and the poll drains the whole inbox.
    #[test]
    fn the_inbox_is_bounded_and_answers_typed_busy() {
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        let handle = SessionHandle {
            epoch: SessionEpoch(1),
            tx,
        };
        handle.play(RequestId(1)).must();
        handle.pause(RequestId(2)).must();
        assert!(matches!(
            handle.stop(RequestId(3)),
            Err(SessionError::InboxBusy)
        ));
        // The queued commands are intact, in order.
        let first = rx.try_recv().must();
        let second = rx.try_recv().must();
        assert_eq!(first.request, RequestId(1));
        assert_eq!(second.request, RequestId(2));
        // After a drain there is room again.
        handle.stop(RequestId(3)).must();
    }
}
