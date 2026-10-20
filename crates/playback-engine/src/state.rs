//! The playback session state machine (card 2.C2; doc 03 §4.6(b),
//! doc 05 §3.3): the 8 wire-visible states, the session inputs, and
//! the transition table as **data**.
//!
//! # What is minimal here (and what is not)
//!
//! Doc 03 §4.6(b) sketches a 10-state `SessionState` (adding `Probing`
//! and `Demuxing`); doc 05 §3.3 — the wire conformance contract, types
//! AND data in the `commands` crate — defines exactly 8. Card 2.C2
//! implements the **minimal machine**: the 8 wire states, with probing
//! and demux-start absorbed as sub-phases of [`SessionState::Opening`]
//! (the open step runs `Demuxer::start` — header parse + track
//! enumeration — which *is* the probe/demux boundary at the trait
//! level). Doc 03's fuller machine is card 3.C1's completeness work.
//!
//! The table below is a strict **subset** of the doc 05 §3.3 wire
//! conformance list (`commands::PlaybackState::can_transition_to`) —
//! proven row-by-row by the exhaustive transition-table tests, so no
//! engine-side transition can ever surface on the wire as a
//! conformance failure. Transitions the wire table does not model in
//! v1 are **rejected with typed reasons**, never silently performed:
//!
//! - `Stop` during `Opening`/`Buffering` and `Close` from
//!   `Buffering`/`Playing`/`Paused` (doc 03 §4.6 has
//!   "any active → Stop → Stopped"; the doc 05 §3.3 wire list has no
//!   such edges — the reconciliation is owed to card 3.C1
//!   "stop-during-opening" and reopen-cycle completeness).
//! - `Open` on a live session (source replacement is card 3.A2).
//! - `Play` in `Ended` (restart-from-ended is the Phase-3 seek flow,
//!   card 3.A1 — doc 05 §2.4 deliberately wires `playback.play` as
//!   "restart-from-ended" only once seek exists).
//!
//! # Applying inputs
//!
//! [`apply_input`] is **pure**: it looks the `(state, input)` pair up
//! in [`TRANSITIONS`] and answers either the matching row (target +
//! effects) or a typed [`TransitionRejected`] with a
//! [`RejectionReason`]. All mutation lives in the session executor
//! ([`crate::session`]), which is the single writer (doc 03 §4.6(e)
//! invariant S1 — one owner, no interior mutability needed in v0).
//! Illegal transitions are therefore not just rejected — they are
//! *unreachable through any other path*, because no other code path
//! mutates session state at all.
//!
//! # Self-transitions (C8)
//!
//! Doc 05 §3.3 allows self-transitions as idempotent command
//! confirmations. The table encodes the ones v0 honors: `Play` while
//! `Playing`/`Buffering`, `Pause` while `Paused`, and `Close` on an
//! already-absent session (`Idle`). Each emits a `state-changed`
//! confirmation event but performs no state work.
//!
//! # Pipeline inputs and the driver guard
//!
//! Inputs are either **commands** ([`SessionInput::Play`],
//! [`Pause`], [`Stop`], [`Close`], plus [`Open`] applied by the engine
//! at construction) or **pipeline signals** ([`OpenReady`],
//! [`Primed`], [`Underrun`], [`EofReached`], [`Drained`],
//! [`Failed`]) raised by the pipeline tick. The tick never applies a
//! signal directly: it goes through
//! [`translate_pipeline_signal`], which maps raw pipeline facts to the
//! one input that is legal for the current state (or `None`). The
//! exhaustive tests prove `translate` only ever yields table-legal
//! inputs — the guard plus the table together are what makes *no
//! invalid transition reachable*.

use commands::PlaybackState;

/// The 8 wire-visible playback states (doc 05 §3.3; card 2.C2's
/// minimal machine — see the module docs for what was folded into
/// [`SessionState::Opening`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    /// No live session (the engine's resting state).
    Idle,
    /// A source is being opened/probed (absorbs doc 03's `Probing`
    /// and `Demuxing` sub-phases in the minimal machine).
    Opening,
    /// Decoders are filling their buffers (initial fill or rebuffer).
    Buffering,
    /// The pipeline is running.
    Playing,
    /// Paused by the user/engine; the clock is frozen.
    Paused,
    /// Current media finished; the session is retained (seek-back is
    /// Phase 3 — v0 exits via `Close` only).
    Ended,
    /// Pipeline torn down; the session survives (doc 05 §2.4
    /// `playback.stop`).
    Stopped,
    /// Terminal failure (always accompanied by a fatal
    /// `playback.error` emitted *before* `state-changed → error`).
    Error,
}

impl SessionState {
    /// Every state, in wire-catalog order (doc 05 §3.3).
    pub const ALL: [SessionState; 8] = [
        SessionState::Idle,
        SessionState::Opening,
        SessionState::Buffering,
        SessionState::Playing,
        SessionState::Paused,
        SessionState::Stopped,
        SessionState::Ended,
        SessionState::Error,
    ];

    /// The wire form (doc 05 §3.3). The mapping is the identity — the
    /// engine state and the wire state share names and order, so
    /// reconciliation is by construction (and pinned by test).
    #[must_use]
    pub fn as_wire(self) -> PlaybackState {
        match self {
            SessionState::Idle => PlaybackState::Idle,
            SessionState::Opening => PlaybackState::Opening,
            SessionState::Buffering => PlaybackState::Buffering,
            SessionState::Playing => PlaybackState::Playing,
            SessionState::Paused => PlaybackState::Paused,
            SessionState::Ended => PlaybackState::Ended,
            SessionState::Stopped => PlaybackState::Stopped,
            SessionState::Error => PlaybackState::Error,
        }
    }
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SessionState::Idle => "idle",
            SessionState::Opening => "opening",
            SessionState::Buffering => "buffering",
            SessionState::Playing => "playing",
            SessionState::Paused => "paused",
            SessionState::Ended => "ended",
            SessionState::Stopped => "stopped",
            SessionState::Error => "error",
        };
        f.write_str(name)
    }
}

/// One input to the state machine: a client command or a pipeline
/// signal (see the module docs). Payloads (the open configuration, the
/// fatal error, the request id) travel beside the input in the
/// executor's context — the table itself stays pure data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionInput {
    /// `session.open` — applied by the engine when constructing the
    /// session (the epoch is minted engine-side before the session
    /// exists). Re-application on a live session is source replacement
    /// (card 3.A2).
    Open,
    /// `playback.play` — play / resume; an idempotent self-confirmation
    /// while already playing or buffering (C8).
    Play,
    /// `playback.pause` — pause; an idempotent self-confirmation while
    /// already paused (C8).
    Pause,
    /// `playback.stop` — tear the pipeline down; the session survives
    /// (gen-bumping).
    Stop,
    /// `session.close` — destroy the session (class S, gen-bumping,
    /// idempotent; see the table's `Idle` self-row).
    Close,
    /// The open step succeeded: the demuxer parsed the container and
    /// the pipeline (queues + decoders) is wired. Raised by the open
    /// step after wiring — the row's effects only announce it.
    OpenReady,
    /// Every enabled limb's packet queue reached its high watermark:
    /// enough buffered data to start (doc 03 §4.6 `Buffering | high
    /// watermark | Playing`).
    Primed,
    /// A limb's queue drained to its low watermark while the demuxer
    /// can still refill (doc 03 §4.6 `Playing | low-watermark underrun
    /// | Buffering`). At end-of-stream the drain is *not* an underrun
    /// — the input is end-of-media instead.
    Underrun,
    /// The demuxer reached end-of-stream with less than the high
    /// watermark queued (doc 03 §4.6 short media: "start with what
    /// exists"). While already `Playing` the fact is absorbed by the
    /// pipeline as its end-of-stream flag (no session input).
    EofReached,
    /// End-of-stream **and** every queue is empty **and** every
    /// decoder has flushed to `Drained`: the media is fully presented
    /// (doc 03 §4.6 `Playing | EOF + queues drained | Ended`).
    Drained,
    /// A fatal, typed failure (open, demux or decode). The engine
    /// emits `playback.error` *before* the `state-changed → error`
    /// event (doc 05 §3.3 ordering).
    Failed,
}

impl SessionInput {
    /// Every input, for exhaustive tests and model walks.
    pub const ALL: [SessionInput; 11] = [
        SessionInput::Open,
        SessionInput::Play,
        SessionInput::Pause,
        SessionInput::Stop,
        SessionInput::Close,
        SessionInput::OpenReady,
        SessionInput::Primed,
        SessionInput::Underrun,
        SessionInput::EofReached,
        SessionInput::Drained,
        SessionInput::Failed,
    ];

    /// Whether this input originates from a client command (as opposed
    /// to a pipeline signal).
    #[must_use]
    pub fn is_command(self) -> bool {
        matches!(
            self,
            SessionInput::Open | SessionInput::Play | SessionInput::Pause | SessionInput::Stop | SessionInput::Close
        )
    }
}

impl std::fmt::Display for SessionInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SessionInput::Open => "open",
            SessionInput::Play => "play",
            SessionInput::Pause => "pause",
            SessionInput::Stop => "stop",
            SessionInput::Close => "close",
            SessionInput::OpenReady => "open-ready",
            SessionInput::Primed => "primed",
            SessionInput::Underrun => "underrun",
            SessionInput::EofReached => "eof-reached",
            SessionInput::Drained => "drained",
            SessionInput::Failed => "failed",
        };
        f.write_str(name)
    }
}

/// One transition-table row: the applied effect set for a legal
/// `(from, input)` pair (doc 03 §4.6's table, "Side effects" column).
///
/// Two side effects of the doc's table are structural rather than
/// row effects and therefore do not appear here: the session **epoch
/// is minted by the engine before the session exists** (the
/// `Idle -- Open --> Opening` row documents it), and the **pipeline
/// is wired by the open step before it raises `OpenReady`** (the
/// input means "wiring succeeded"). Every applied transition also
/// emits a `state-changed` event (doc 05 §3.2: never coalesced, C8
/// confirmations included) — that emission is the executor's
/// standard postlude, not a per-row effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// The state the machine is in.
    pub from: SessionState,
    /// The applied input.
    pub input: SessionInput,
    /// The state the machine moves to (may equal `from` — C8).
    pub to: SessionState,
    /// The side effects the executor performs, in order.
    pub effects: &'static [Effect],
}

/// The side effects a transition carries (doc 03 §4.6, "Side
/// effects" column, minimal-machine subset). The executor interprets
/// them; the table only carries them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Emit the fatal `playback.error` for the pending failure —
    /// always *before* the `state-changed → error` event (doc 05
    /// §3.3 ordering).
    ReportFatal,
    /// Destroy the session: interrupt the cancel token, close the
    /// queues, drop the pipeline; the engine forgets the session
    /// (targets `Idle`).
    DestroySession,
    /// Mint the next pipeline generation (doc 03 §5: the final bump of
    /// a `stop`/`close`) and publish it to the cancel token.
    BumpPipeline,
    /// Flush every packet queue with the new generation and close it,
    /// then drop the decoders and the demuxer (the v0 synchronous
    /// form of doc 03 §5's teardown order: mint → interrupt → flush →
    /// join; there are no worker threads to join yet).
    TeardownPipeline,
    /// Freeze the playback clock (pause / underrun entry).
    FreezeClock,
    /// Un-pause the playback clock (playing entry).
    ResumeClock,
    /// Freeze the clock at the end position (ended entry; v0 pauses —
    /// a distinct "stopped at end" anchor state is 2.C3/3.B2 work).
    StopClock,
    /// Emit the initial track list (`tracks.list-ready`) — delivered
    /// before the session's first `state-changed → playing` (doc 05
    /// §4.3 terminal invariant).
    EmitTracks,
    /// Emit `session.buffering` with progress `0.0` (doc 05 §3.2: an
    /// episode must open with `0.0`).
    EmitBufferingEnter,
    /// Emit `session.buffering` with progress `1.0` (doc 05 §3.2: an
    /// episode must close with `1.0`).
    EmitBufferingExit,
    /// Emit `playback.end-of-media` — before the `state-changed →
    /// ended` event (doc 05 §3.2).
    EmitEndOfMedia,
}

/// The minimal-machine transition table (23 rows; see the module
/// docs). Data, not code — the exhaustive tests classify every
/// `(state, input)` pair against it.
pub const TRANSITIONS: &[Transition] = &[
    // ---- open ----
    // The engine mints the session epoch before constructing the
    // session, then applies this row (doc 03 §4.6: "mint session
    // epoch; spawn input thread with open deadline" — v0's open step
    // is the synchronous `Demuxer::start` on the poll thread).
    Transition {
        from: SessionState::Idle,
        input: SessionInput::Open,
        to: SessionState::Opening,
        effects: &[],
    },
    // Idempotent close on an already-absent session (doc 05 §2.3:
    // session.close is idempotent). Reached engine-side when no
    // session exists; kept as data so the executor's confirmation
    // semantics are uniform.
    Transition {
        from: SessionState::Idle,
        input: SessionInput::Close,
        to: SessionState::Idle,
        effects: &[],
    },
    // ---- open completes / fails ----
    // The open step has already wired queues and decoders; the row
    // announces the tracks and opens the buffering episode.
    Transition {
        from: SessionState::Opening,
        input: SessionInput::OpenReady,
        to: SessionState::Buffering,
        effects: &[Effect::EmitTracks, Effect::EmitBufferingEnter],
    },
    // doc 03 §4.6 `Opening | input error / deadline | Error` — "typed
    // Error(...); teardown": the open failed before any limb was wired,
    // so the teardown drops the (limb-less) pipeline. Entering Error
    // tears down as stopped would (doc 03 §4.6 "any active | fatal
    // error | Error ... teardown as Stopped").
    Transition {
        from: SessionState::Opening,
        input: SessionInput::Failed,
        to: SessionState::Error,
        effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
    },
    // doc 05 §3.3's `opening → idle` edge: close while opening.
    Transition {
        from: SessionState::Opening,
        input: SessionInput::Close,
        to: SessionState::Idle,
        effects: &[Effect::BumpPipeline, Effect::DestroySession],
    },
    // ---- buffering ----
    Transition {
        from: SessionState::Buffering,
        input: SessionInput::Primed,
        to: SessionState::Playing,
        effects: &[Effect::ResumeClock, Effect::EmitBufferingExit],
    },
    // Short media (doc 03 §4.6): EOF before the high watermark —
    // start with what exists, then drain quickly to `Ended`.
    Transition {
        from: SessionState::Buffering,
        input: SessionInput::EofReached,
        to: SessionState::Playing,
        effects: &[Effect::ResumeClock, Effect::EmitBufferingExit],
    },
    // C8: play while still priming is an idempotent confirmation.
    Transition {
        from: SessionState::Buffering,
        input: SessionInput::Play,
        to: SessionState::Buffering,
        effects: &[],
    },
    Transition {
        from: SessionState::Buffering,
        input: SessionInput::Failed,
        to: SessionState::Error,
        effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
    },
    // ---- playing ----
    // C8: play while playing.
    Transition {
        from: SessionState::Playing,
        input: SessionInput::Play,
        to: SessionState::Playing,
        effects: &[],
    },
    Transition {
        from: SessionState::Playing,
        input: SessionInput::Pause,
        to: SessionState::Paused,
        effects: &[Effect::FreezeClock],
    },
    // Rebuffer (doc 03 §4.6 `Playing | low-watermark underrun`).
    Transition {
        from: SessionState::Playing,
        input: SessionInput::Underrun,
        to: SessionState::Buffering,
        effects: &[Effect::FreezeClock, Effect::EmitBufferingEnter],
    },
    Transition {
        from: SessionState::Playing,
        input: SessionInput::Drained,
        to: SessionState::Ended,
        effects: &[Effect::StopClock, Effect::EmitEndOfMedia],
    },
    // doc 03 §4.6 `any active | Stop`: v0 honors it from the two
    // states the wire table models (see module docs — the transient
    // states are card 3.C1's).
    Transition {
        from: SessionState::Playing,
        input: SessionInput::Stop,
        to: SessionState::Stopped,
        effects: &[Effect::BumpPipeline, Effect::TeardownPipeline],
    },
    Transition {
        from: SessionState::Playing,
        input: SessionInput::Failed,
        to: SessionState::Error,
        effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
    },
    // ---- paused ----
    Transition {
        from: SessionState::Paused,
        input: SessionInput::Play,
        to: SessionState::Playing,
        effects: &[Effect::ResumeClock],
    },
    // C8: pause while paused.
    Transition {
        from: SessionState::Paused,
        input: SessionInput::Pause,
        to: SessionState::Paused,
        effects: &[],
    },
    // End-of-media observed while paused (doc 05 §3.3 allows
    // `paused → ended`): everything was already drained when the
    // pause landed; the end signal consumes nothing.
    Transition {
        from: SessionState::Paused,
        input: SessionInput::Drained,
        to: SessionState::Ended,
        effects: &[Effect::StopClock, Effect::EmitEndOfMedia],
    },
    Transition {
        from: SessionState::Paused,
        input: SessionInput::Stop,
        to: SessionState::Stopped,
        effects: &[Effect::BumpPipeline, Effect::TeardownPipeline],
    },
    Transition {
        from: SessionState::Paused,
        input: SessionInput::Failed,
        to: SessionState::Error,
        effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
    },
    // ---- settled states: close/replace exits (doc 05 §3.3) ----
    Transition {
        from: SessionState::Ended,
        input: SessionInput::Close,
        to: SessionState::Idle,
        effects: &[Effect::BumpPipeline, Effect::DestroySession],
    },
    Transition {
        from: SessionState::Stopped,
        input: SessionInput::Close,
        to: SessionState::Idle,
        effects: &[Effect::BumpPipeline, Effect::DestroySession],
    },
    Transition {
        from: SessionState::Error,
        input: SessionInput::Close,
        to: SessionState::Idle,
        effects: &[Effect::BumpPipeline, Effect::DestroySession],
    },
];

/// Why an input was refused for a state (typed rejection — doc 03
/// §4.6: `CommandRejected{reason}`, never an error crash).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionReason {
    /// There is no live session to do this work (the engine is
    /// `Idle`).
    NoSession,
    /// The session is still opening; the pipeline is not wired yet.
    NotReady,
    /// The v1 wire conformance table (doc 05 §3.3) has no edge from
    /// this state to `stopped`/`idle` — stop-or-close from a transient
    /// or active state lands with card 3.C1 (stop-during-opening and
    /// active-close completeness). The rejection is deliberate, not an
    /// omission.
    StopFirst,
    /// The work is already done (`stop`/`pause` after the session
    /// settled); `close` is the remaining exit.
    AlreadySettled,
    /// `play` in `Ended` means restart-from-ended, which is the
    /// Phase-3 seek flow (card 3.A1); v0 cannot honor it without
    /// seeking.
    RestartNeedsSeek,
    /// `open` on a live session is source replacement (card 3.A2);
    /// open a new engine session after closing this one.
    SourceReplacementPhase3,
    /// A pipeline signal in a state whose driver never raises it
    /// ([`translate_pipeline_signal`] yields `None` there). Reaching
    /// this reason at runtime means a caller bypassed the driver
    /// guard — a bug report, not a media condition.
    SignalNotRaisedHere,
}

impl std::fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            RejectionReason::NoSession => "no live session for this input",
            RejectionReason::NotReady => "session is still opening",
            RejectionReason::StopFirst => {
                "stop-or-close from this state is not wire-conformant in v1 (owed to card 3.C1)"
            }
            RejectionReason::AlreadySettled => "the work is already done; close the session",
            RejectionReason::RestartNeedsSeek => {
                "restart-from-ended needs the Phase-3 seek flow (card 3.A1)"
            }
            RejectionReason::SourceReplacementPhase3 => {
                "source replacement on a live session is card 3.A2"
            }
            RejectionReason::SignalNotRaisedHere => {
                "pipeline signal not raised in this state by the driver"
            }
        };
        f.write_str(text)
    }
}

/// The typed rejection of an illegal `(state, input)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRejected {
    /// The state the machine was in.
    pub from: SessionState,
    /// The refused input.
    pub input: SessionInput,
    /// Why it was refused.
    pub reason: RejectionReason,
}

impl std::fmt::Display for TransitionRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is not applicable in {} ({})",
            self.input, self.from, self.reason
        )
    }
}

/// The pure heart of the machine: look `(state, input)` up in
/// [`TRANSITIONS`].
///
/// This function is the **only** way session state moves (the session
/// executor calls it; nothing else mutates state), so an illegal
/// transition is unreachable by construction: every pair is either a
/// table row or a typed rejection, and the exhaustive tests pin all
/// `8 × 11 = 88` classifications.
///
/// # Errors
/// [`TransitionRejected`] when no row matches — with the pair's
/// [`RejectionReason`] (see [`rejection_reason`]).
pub fn apply_input(
    state: SessionState,
    input: SessionInput,
) -> Result<&'static Transition, TransitionRejected> {
    match TRANSITIONS
        .iter()
        .find(|row| row.from == state && row.input == input)
    {
        Some(row) => Ok(row),
        None => Err(TransitionRejected {
            from: state,
            input,
            reason: rejection_reason(state, input),
        }),
    }
}

/// The typed reason for a rejected pair — total over every
/// `(state, input)` combination that is not a table row (also total
/// over rows, where the answer is simply never consulted).
#[must_use]
pub fn rejection_reason(state: SessionState, input: SessionInput) -> RejectionReason {
    use RejectionReason as R;
    use SessionInput as I;
    use SessionState as S;
    match (state, input) {
        // `open` anywhere but a fresh engine = replacement (3.A2).
        (_, I::Open) => R::SourceReplacementPhase3,
        // No session work exists in Idle.
        (S::Idle, I::Play | I::Pause | I::Stop) => R::NoSession,
        // Pipeline signals cannot be raised where there is no
        // pipeline.
        (S::Idle, I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed) => {
            R::SignalNotRaisedHere
        }
        // Opening: commands the pipeline cannot honor yet, plus the
        // two deliberately-absent v1 edges.
        (S::Opening, I::Play | I::Pause) => R::NotReady,
        (S::Opening, I::Stop) => R::StopFirst,
        (S::Opening, I::Primed | I::Underrun | I::EofReached | I::Drained) => {
            R::SignalNotRaisedHere
        }
        // Buffering: pause-during-buffering has no v1 wire edge
        // (buffering → paused is not in the conformance list — a
        // buffered-pause semantic is owed to 3.C1 with the rest).
        (S::Buffering, I::Pause) => R::NotReady,
        (S::Buffering, I::Stop | I::Close) => R::StopFirst,
        (S::Buffering, I::OpenReady | I::Underrun | I::Drained) => R::SignalNotRaisedHere,
        // Playing: close requires settling first (no playing → idle
        // edge in v1); steady-state signals are absorbed by the
        // pipeline, not applied to the machine.
        (S::Playing, I::Close) => R::StopFirst,
        (S::Playing, I::OpenReady | I::Primed | I::EofReached) => R::SignalNotRaisedHere,
        // Paused: close requires stop first; underrun cannot be
        // observed with consumption frozen.
        (S::Paused, I::Close) => R::StopFirst,
        (S::Paused, I::OpenReady | I::Underrun | I::Primed | I::EofReached) => R::SignalNotRaisedHere,
        // Ended: retained for Phase-3 seek-back; otherwise inert.
        (S::Ended, I::Play) => R::RestartNeedsSeek,
        (S::Ended, I::Pause | I::Stop) => R::AlreadySettled,
        (S::Ended, I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed) => {
            R::SignalNotRaisedHere
        }
        // Stopped: the session survives, inert until close.
        (S::Stopped, I::Play | I::Pause | I::Stop) => R::AlreadySettled,
        (S::Stopped, I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed) => {
            R::SignalNotRaisedHere
        }
        // Error: inert by invariant S2 (teardown as stopped already
        // happened; the typed error is on the event stream).
        (S::Error, I::Play | I::Pause | I::Stop) => R::AlreadySettled,
        (S::Error, I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed) => {
            R::SignalNotRaisedHere
        }
        // Everything else is a table row (never consulted here, but
        // the match must be total without a wildcard to force a
        // compile error when rows or inputs are added).
        (S::Idle, I::Close)
        | (S::Opening, I::Close | I::OpenReady | I::Failed)
        | (S::Buffering, I::Play | I::Primed | I::EofReached | I::Failed)
        | (S::Playing, I::Play | I::Pause | I::Underrun | I::Drained | I::Stop | I::Failed)
        | (S::Paused, I::Play | I::Pause | I::Drained | I::Stop | I::Failed)
        | (S::Ended, I::Close)
        | (S::Stopped, I::Close)
        | (S::Error, I::Close) => R::SignalNotRaisedHere,
    }
}

/// A raw pipeline fact, as reported by one pipeline tick (the names
/// describe what the *pipeline* observed; the session decides what
/// they mean for the state machine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineSignal {
    /// A fatal, typed pipeline failure (demux or decode error).
    Fatal,
    /// End-of-stream observed, all queues empty, all decoders drained
    /// — the media is fully presented.
    Drained,
    /// The demuxer returned end-of-stream (data may still be queued).
    EofReached,
    /// Every enabled limb's queue is at its high watermark.
    Primed,
    /// Some enabled limb's queue is at its low watermark while the
    /// demuxer can still refill (an underrun, not an end-of-stream
    /// drain).
    Starved,
}

/// The driver guard: translate a raw pipeline signal into the session
/// input that is legal for `state`, or `None` when the fact is
/// absorbed (steady state, no session-level meaning).
///
/// The pipeline tick applies **at most one** translated input per
/// poll, in the priority order `Fatal > Drained > EofReached >
/// `Primed` > `Starved` `[PROPOSAL: keeps one poll's causality linear
/// and its work bounded; Phase 3 may interleave]`. The exhaustive
/// tests prove every `Some` answer is a table row for `state` — this
/// guard plus [`apply_input`] are what make illegal transitions
/// unreachable from the pipeline side.
#[must_use]
pub fn translate_pipeline_signal(
    state: SessionState,
    signal: PipelineSignal,
) -> Option<SessionInput> {
    use SessionInput as I;
    use SessionState as S;
    match (state, signal) {
        (S::Buffering | S::Playing | S::Paused, PipelineSignal::Fatal) => Some(I::Failed),
        (S::Playing | S::Paused, PipelineSignal::Drained) => Some(I::Drained),
        // In Buffering the drain is preempted by the short-media
        // start (EofReached fires first); the playing/paused cases
        // above are the only ones the driver raises.
        (S::Buffering, PipelineSignal::Drained) => None,
        (S::Buffering, PipelineSignal::EofReached) => Some(I::EofReached),
        (S::Buffering, PipelineSignal::Primed) => Some(I::Primed),
        (S::Playing, PipelineSignal::Starved) => Some(I::Underrun),
        // Absorbed facts: end-of-stream while already playing/paused
        // becomes the pipeline's eof flag; a low watermark while
        // buffering is the buffering steady state; a high watermark
        // while playing/paused is the normal filled state. No
        // pipeline exists in the remaining states.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_size_is_pinned() {
        // 23 rows; adding one is a deliberate act (worklog + doc 03
        // §4.6 reference in the row's comment).
        assert_eq!(TRANSITIONS.len(), 23);
    }

    #[test]
    fn state_display_uses_wire_names() {
        assert_eq!(SessionState::Buffering.to_string(), "buffering");
        assert_eq!(SessionState::Error.to_string(), "error");
    }
}
