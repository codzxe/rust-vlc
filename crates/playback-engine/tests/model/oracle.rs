//! The imperative oracle (card 3.C1, the model-based AC's second
//! author): `model_answer` encodes the **documented** state machine —
//! doc 03 §4.6's rows and doc 05 §3.3's wire conformance list — as
//! straight-line imperative code, one match arm per documented rule,
//! each arm citing its source. It is deliberately NOT generated from
//! (or compared against, in its authorship) the engine's
//! [`TRANSITIONS`] data table: the whole point of the three-way proof
//! (`tests/model_check.rs`) is that two authors writing the same
//! spec-derived truth must agree, killing the self-referential echo
//! chamber where a table is only ever checked against itself
//! (directive D-5).
//!
//! Totality discipline (mirroring `rejection_reason`'s, state.rs):
//! the match has **no wildcard** — every `(state, input)` pair is
//! spelled out, so adding an input (a future `SwitchTrack`, say) or
//! a state is a compile error here until this oracle classifies it.
//!
//! The shape is data-free imperative: no table lookup, no iteration
//! over rows — just the rules as the documents state them, in
//! documentation order (open → open-settles → buffering → playing →
//! paused → settled), so a reader can diff the arms against the doc
//! prose directly.

use playback_engine::state::{Effect, RejectionReason, SessionInput, SessionState};

/// The oracle's answer for one `(state, input)` pair: the row the
/// documents mandate, or the typed rejection they imply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelAnswer {
    /// A documented transition: the landing state and the doc 03
    /// §4.6 side effects (as the engine spells them).
    Row {
        to: SessionState,
        effects: &'static [Effect],
    },
    /// Not a documented transition: the typed reason.
    Reject(RejectionReason),
}

/// The documented answer for `(state, input)`.
///
/// Every arm carries its citation: doc 03 §4.6 row (the machine's
/// own table, `state | event | next | side effects`) or doc 05 §3.3
/// (the wire conformance list that bounds which edges may exist at
/// all). Rejection arms cite the reason's own contract.
#[must_use]
pub fn model_answer(state: SessionState, input: SessionInput) -> ModelAnswer {
    use ModelAnswer::{Reject, Row};
    use SessionInput as I;
    use SessionState as S;

    match (state, input) {
        // ---- doc 03 §4.6 "Idle | open | Opening | mint session
        // epoch" (the engine mints before constructing the session,
        // then applies this row; side effects: none — the open step
        // itself is executor work).
        (S::Idle, I::Open) => Row {
            to: S::Opening,
            effects: &[],
        },
        // ---- doc 05 §2.3: session.close is idempotent — close with
        // no session is the C8 confirmation self-row.
        (S::Idle, I::Close) => Row {
            to: S::Idle,
            effects: &[],
        },
        // ---- doc 03 §4.6 "Idle | play/pause/stop/seek | —": no
        // session exists to do the work.
        (S::Idle, I::Play | I::Pause | I::Stop | I::Seek) => Reject(RejectionReason::NoSession),
        // ---- No pipeline exists in Idle: none of the signals can be
        // raised here (doc 03 §4.6(e) S1 — the executor is the only
        // mover, and it never polls in Idle).
        (
            S::Idle,
            I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed,
        ) => Reject(RejectionReason::SignalNotRaisedHere),

        // ---- doc 03 §4.6 "Opening | open-ready | Buffering |
        // announce tracks; start buffering episode".
        (S::Opening, I::OpenReady) => Row {
            to: S::Buffering,
            effects: &[Effect::EmitTracks, Effect::EmitBufferingEnter],
        },
        // ---- `open` on a live session is source replacement
        // (card 3.A2's flow, executor-side; the table's typed
        // stand-in names the card).
        (S::Opening, I::Open) => Reject(RejectionReason::SourceReplacementPhase3),
        // ---- doc 03 §4.6 "Opening | input error / deadline |
        // Error | typed Error(...); teardown".
        (S::Opening, I::Failed) => Row {
            to: S::Error,
            effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
        },
        // ---- doc 05 §3.3 `opening → idle`: close while opening.
        (S::Opening, I::Close) => Row {
            to: S::Idle,
            effects: &[Effect::BumpPipeline, Effect::DestroySession],
        },
        // ---- doc 03 §4.6 "Opening | stop | Stopped | cancel token;
        // join threads" (card 3.C1's stop-during-opening row; the
        // teardown is the Option no-op — the open step never wired a
        // pipeline on this side of the row, but the mint still
        // publishes the cancel token).
        (S::Opening, I::Stop) => Row {
            to: S::Stopped,
            effects: &[Effect::BumpPipeline, Effect::TeardownPipeline],
        },
        // ---- doc 03 §4.6 "Opening | play/pause/seek | —": the
        // pipeline is not wired yet; the commands cannot be honored.
        (S::Opening, I::Play | I::Pause | I::Seek) => Reject(RejectionReason::NotReady),
        // ---- No pipeline is wired in Opening: the signals cannot be
        // raised (the open step wires at OpenReady).
        (
            S::Opening,
            I::Primed | I::Underrun | I::EofReached | I::Drained,
        ) => Reject(RejectionReason::SignalNotRaisedHere),

        // ---- doc 03 §4.6 "Buffering | high watermark | Playing |
        // resume clock; close buffering episode (1.0)".
        (S::Buffering, I::Primed) => Row {
            to: S::Playing,
            effects: &[Effect::ResumeClock, Effect::EmitBufferingExit],
        },
        // ---- Source replacement (3.A2's typed stand-in).
        (S::Buffering, I::Open) => Reject(RejectionReason::SourceReplacementPhase3),
        // ---- doc 03 §4.6 short media: "EOF before the high
        // watermark — start with what exists" → Playing, the episode
        // closes (1.0) and the clock resumes.
        (S::Buffering, I::EofReached) => Row {
            to: S::Playing,
            effects: &[Effect::ResumeClock, Effect::EmitBufferingExit],
        },
        // ---- C8 (doc 05 §3.3 self-transitions): play while still
        // priming is the idempotent confirmation.
        (S::Buffering, I::Play) => Row {
            to: S::Buffering,
            effects: &[],
        },
        // ---- doc 03 §4.6 "Buffering | pause | Paused" (card 3.C1's
        // VLC-parity buffered pause): the clock freezes (idempotent —
        // it is not running during initial fill) and the episode
        // stays open; the `Paused + Primed` bookend row (or the
        // resume) closes it.
        (S::Buffering, I::Pause) => Row {
            to: S::Paused,
            effects: &[Effect::FreezeClock],
        },
        // ---- doc 03 §4.6 "Buffering | stop | Stopped | flush, join"
        // (card 3.C1): the settling exit closes the mid-episode
        // buffering episode with its 1.0 bookend before the
        // `state-changed → stopped` (doc 05 §3.2).
        (S::Buffering, I::Stop) => Row {
            to: S::Stopped,
            effects: &[
                Effect::BumpPipeline,
                Effect::TeardownPipeline,
                Effect::EmitBufferingExit,
            ],
        },
        // ---- doc 03 §4.6 "any active | fatal error | Error | typed
        // Error(...); teardown as Stopped".
        (S::Buffering, I::Failed) => Row {
            to: S::Error,
            effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
        },
        // ---- Card 3.A1 (doc 03 §6): a seek landed during initial
        // buffering re-runs the flow from step 1 — a work-carrying
        // self-transition (generation bump + pipeline re-anchor), no
        // new wire edge.
        (S::Buffering, I::Seek) => Row {
            to: S::Buffering,
            effects: &[Effect::BumpPipeline, Effect::SeekPipeline],
        },
        // ---- Close from Buffering composes through the executor's
        // stop-then-close macro (`close_plan`): the pure table keeps
        // the typed pair (doc 05 §3.3 has no `buffering → idle`
        // edge), but no client path can reach it.
        (S::Buffering, I::Close) => Reject(RejectionReason::SettleFirst),
        // ---- doc 03 §4.6 "Buffering | play | —" is the C8 row
        // above; the remaining signals are either absorbed by the
        // pipeline as facts (underrun while buffering is the steady
        // state; drained is preempted by the short-media start) or
        // cannot be raised (open-ready fires once, in Opening).
        (S::Buffering, I::OpenReady | I::Underrun | I::Drained) => {
            Reject(RejectionReason::SignalNotRaisedHere)
        }

        // ---- C8: play while playing.
        (S::Playing, I::Play) => Row {
            to: S::Playing,
            effects: &[],
        },
        // ---- Source replacement (3.A2's typed stand-in).
        (S::Playing, I::Open) => Reject(RejectionReason::SourceReplacementPhase3),
        // ---- doc 03 §4.6 "Playing | pause | Paused | freeze clock".
        (S::Playing, I::Pause) => Row {
            to: S::Paused,
            effects: &[Effect::FreezeClock],
        },
        // ---- doc 03 §4.6 "Playing | low-watermark underrun |
        // Buffering | freeze clock; open buffering episode".
        (S::Playing, I::Underrun) => Row {
            to: S::Buffering,
            effects: &[Effect::FreezeClock, Effect::EmitBufferingEnter],
        },
        // ---- doc 03 §4.6 "Playing | EOF + queues drained | Ended |
        // stop clock; end-of-media".
        (S::Playing, I::Drained) => Row {
            to: S::Ended,
            effects: &[Effect::StopClock, Effect::EmitEndOfMedia],
        },
        // ---- doc 03 §6 (card 3.A1): the seek flow's wire-visible
        // shape from Playing is the rebuffer episode — freeze, bump,
        // re-anchor, open the episode.
        (S::Playing, I::Seek) => Row {
            to: S::Buffering,
            effects: &[
                Effect::FreezeClock,
                Effect::BumpPipeline,
                Effect::SeekPipeline,
                Effect::EmitBufferingEnter,
            ],
        },
        // ---- doc 03 §4.6 "any active | stop | Stopped" — v0 honors
        // it from Playing (and, since 3.C1, Opening/Buffering/Paused).
        (S::Playing, I::Stop) => Row {
            to: S::Stopped,
            effects: &[Effect::BumpPipeline, Effect::TeardownPipeline],
        },
        // ---- doc 03 §4.6 "any active | fatal error | Error".
        (S::Playing, I::Failed) => Row {
            to: S::Error,
            effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
        },
        // ---- Close from Playing composes through stop (no
        // `playing → idle` wire edge, doc 05 §3.3; the macro applies).
        (S::Playing, I::Close) => Reject(RejectionReason::SettleFirst),
        // ---- Steady-state facts absorbed by the pipeline (doc 03
        // §4.6: the high watermark while playing is the normal filled
        // state; EOF while playing is the pipeline's eof flag — the
        // session input arrives only at the drained point).
        (S::Playing, I::OpenReady | I::Primed | I::EofReached) => {
            Reject(RejectionReason::SignalNotRaisedHere)
        }

        // ---- doc 03 §4.6 "Paused | play | Playing | resume clock"
        // (card 3.C1: a resume out of a held episode also closes the
        // episode with its 1.0 — executor discipline, pinned by the
        // contract suite).
        (S::Paused, I::Play) => Row {
            to: S::Playing,
            effects: &[Effect::ResumeClock],
        },
        // ---- Source replacement (3.A2's typed stand-in).
        (S::Paused, I::Open) => Reject(RejectionReason::SourceReplacementPhase3),
        // ---- C8: pause while paused.
        (S::Paused, I::Pause) => Row {
            to: S::Paused,
            effects: &[],
        },
        // ---- Card 3.C1's episode bookend: the queues reached their
        // high watermark while the pause holds — the buffered pause's
        // episode closes at 1.0, the session stays paused (a
        // work-carrying self-transition, not a C8 idempotence).
        (S::Paused, I::Primed) => Row {
            to: S::Paused,
            effects: &[Effect::EmitBufferingExit],
        },
        // ---- doc 05 §3.3 allows `paused → ended`: end-of-media
        // observed while paused (everything already drained).
        (S::Paused, I::Drained) => Row {
            to: S::Ended,
            effects: &[Effect::StopClock, Effect::EmitEndOfMedia],
        },
        // ---- doc 03 §6: seek while paused stays paused (VLC parity)
        // — the work-carrying self-row.
        (S::Paused, I::Seek) => Row {
            to: S::Paused,
            effects: &[Effect::BumpPipeline, Effect::SeekPipeline],
        },
        // ---- doc 03 §4.6 "any active | stop | Stopped".
        (S::Paused, I::Stop) => Row {
            to: S::Stopped,
            effects: &[Effect::BumpPipeline, Effect::TeardownPipeline],
        },
        // ---- doc 03 §4.6 "any active | fatal error | Error".
        (S::Paused, I::Failed) => Row {
            to: S::Error,
            effects: &[Effect::ReportFatal, Effect::TeardownPipeline],
        },
        // ---- Close from Paused composes through stop (the macro).
        (S::Paused, I::Close) => Reject(RejectionReason::SettleFirst),
        // ---- Underrun cannot be observed with consumption frozen;
        // open-ready/eof are pipeline-absorbed facts here.
        (S::Paused, I::OpenReady | I::Underrun | I::EofReached) => {
            Reject(RejectionReason::SignalNotRaisedHere)
        }

        // ---- doc 05 §3.3 `stopped → idle` (close/replace).
        (S::Stopped, I::Close) => Row {
            to: S::Idle,
            effects: &[Effect::BumpPipeline, Effect::DestroySession],
        },
        // ---- Source replacement (3.A2's typed stand-in).
        (S::Stopped, I::Open) => Reject(RejectionReason::SourceReplacementPhase3),
        // ---- The work is done (doc 05 §2.4: `playback.stop` parks
        // the session; only `close` remains).
        (S::Stopped, I::Play | I::Pause | I::Stop | I::Seek) => {
            Reject(RejectionReason::AlreadySettled)
        }
        // ---- No pipeline exists in Stopped.
        (
            S::Stopped,
            I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed,
        ) => Reject(RejectionReason::SignalNotRaisedHere),

        // ---- doc 05 §3.3 `ended → idle`.
        (S::Ended, I::Close) => Row {
            to: S::Idle,
            effects: &[Effect::BumpPipeline, Effect::DestroySession],
        },
        // ---- Source replacement (3.A2's typed stand-in).
        (S::Ended, I::Open) => Reject(RejectionReason::SourceReplacementPhase3),
        // ---- doc 05 §2.4: restart-from-ended is deliberately not
        // wired until the ended-seek-back flow (3.B2's scope).
        (S::Ended, I::Play) => Reject(RejectionReason::RestartNeedsSeek),
        // ---- doc 03 §4.6 has `Ended | seek | Buffering`, but doc 05
        // §3.3 has no `ended → buffering` edge — the reconciliation
        // is Lead-owned (the typed stand-in).
        (S::Ended, I::Seek) => Reject(RejectionReason::EndedNeedsWireEdge),
        // ---- Settled: stop/pause are already done.
        (S::Ended, I::Pause | I::Stop) => Reject(RejectionReason::AlreadySettled),
        // ---- No pipeline exists in Ended.
        (
            S::Ended,
            I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed,
        ) => Reject(RejectionReason::SignalNotRaisedHere),

        // ---- doc 05 §3.3 `error → idle` (the closeable terminal).
        (S::Error, I::Close) => Row {
            to: S::Idle,
            effects: &[Effect::BumpPipeline, Effect::DestroySession],
        },
        // ---- Source replacement (3.A2's typed stand-in).
        (S::Error, I::Open) => Reject(RejectionReason::SourceReplacementPhase3),
        // ---- The terminal is closeable and nothing else (doc 03
        // §4.6: "the only exit from Error is Close").
        (S::Error, I::Play | I::Pause | I::Stop | I::Seek) => {
            Reject(RejectionReason::AlreadySettled)
        }
        // ---- No pipeline exists in Error.
        (
            S::Error,
            I::OpenReady | I::Primed | I::Underrun | I::EofReached | I::Drained | I::Failed,
        ) => Reject(RejectionReason::SignalNotRaisedHere),
    }
}
