//! The seek flow (card 3.A1; doc 03 §6): the seven steps, the
//! failed-seek contract **D4**, and the in-flight completion
//! machinery.
//!
//! # The seven steps, as implemented
//!
//! 1. **Accept** — the [`Seek`] command is validated twice (doc 03
//!    §4.6): the state table refuses it in states with no seekable
//!    pipeline ([`crate::state::apply_input`], typed
//!    [`RejectionReason`]); a table-legal command is then
//!    semantically validated here — target resolution (clamping is
//!    truth, doc 05 §2.4) and the demuxer's
//!    [`SeekSupport`](plugin_api::SeekSupport). A validated seek
//!    answers `Accepted { gen: new }` (class A: accepted ≠ completed).
//!    An unresolvable target or an unseekable source emits
//!    [`EngineEventKind::SeekFailed`](crate::event::EngineEventKind)
//!    immediately — **never a fake success** (MP §8 bullet 4).
//! 2. **Bump** — the row's `BumpPipeline` effect mints pipeline
//!    generation G+1 and publishes the token (doc 05 §4.2: the wire
//!    `gen` moves at accept time; the receipt carries the new one).
//! 3. **Flush pipeline** — [`Pipeline::seek_flush`] (called by this
//!    module's [`pipeline_seek`]): every packet queue is flushed with
//!    the new generation (stale residents die at the boundary,
//!    packet-queue Q2), the held demux packet is dropped, the
//!    end-of-stream flag resets (a backward seek revives an exhausted
//!    stream), every decoder is flushed to `Drained` with a
//!    generation-checking sink (its old-generation buffered frames
//!    are dropped and counted — doc 03 §5 checking point 5) and then
//!    reconfigured for the new generation (the codec-api seek
//!    contract). The clock was frozen at the **old** position by the
//!    row's `FreezeClock` effect (the UI shows a seeking state, not a
//!    fabricated new one).
//! 4. **Demux re-anchor** — [`Pipeline::demuxer_reanchor`] positions
//!    the demuxer at its keyframe-safe position ≤ the target and
//!    reports the actual landing A (containers that can only seek
//!    approximately report the truth). Success records the
//!    [`InFlightSeek`]; steps 5–7 continue on the tick path.
//! 5. **Re-prime** — the pipeline ticks resume demux from A; the
//!    queues fill and decoders deliver the landing's frames.
//!    Completion is observed by [`Pipeline::seek_preroll`]: every
//!    enabled limb has delivered ≥ 1 frame of the new generation,
//!    or can provably never deliver (its queue is empty at
//!    end-of-stream with the decoder already `Drained`). A landing
//!    where every limb provably never delivers still completes —
//!    the alternative is a seek that never reports, and the drained
//!    signal then ends the media honestly at the landing instead.
//!    `[PROPOSAL: doc 03 §6 step 5 words the audio condition as
//!    "the first audio packet is queued"; v0 waits for the first
//!    audio *frame* — the frame's own timestamp is the step-6
//!    anchor truth, and the decode slice that produces it runs in
//!    the same tick the packet queues.]`
//! 6. **Clock re-anchor** — [`complete_seek`]:
//!    `clock.reanchor(A_adjusted)` where `A_adjusted` is the first
//!    **audio** frame's own timestamp when an audio limb exists
//!    (anchor on truth, not on request), otherwise A.
//! 7. **Report** — [`EngineEventKind::Seeked`] carries
//!    `{requested, actual}` — the completion truth (doc 05 §3.2
//!    `timing.seek-completed`); the wire-visible state may still be
//!    `buffering` for a tick or two until the priming watermark
//!    re-enters `Playing` through the existing machinery.
//!
//! # Contract D4 — failed seeks
//!
//! On any failure the session **rolls back to the prior position**:
//! the demuxer's `reanchor` is compute-then-commit (a failed call
//! leaves it at its pre-seek position with a valid parsing state —
//! plugin-api invariant D4), the clock never moved (it re-anchors
//! only at step 6, on success), and the flushed pipeline simply
//! re-fills from where the demuxer still stands (a rebuffer — honest,
//! bounded). The typed failure is surfaced as
//! [`EngineEventKind::SeekFailed`] with the old position; no
//! [`EngineEventKind::Seeked`] is ever emitted for a failed
//! generation. If the demuxer is broken (not merely refusing), the
//! next pipeline tick surfaces the fatal through the existing
//! `Failed` path — degraded explicitly, never a false success.
//!
//! # Superseded seeks
//!
//! A second `Seek` accepted while one is in flight re-runs from step
//! 1 against the in-flight generation (the `Buffering + Seek` row):
//! the older seek is replaced in the session's single
//! [`InFlightSeek`] slot and therefore emits **no terminal event**
//! (doc 05 §4.2 — superseded seeks are silent). Seek terminals are
//! exempt from the outbox's generation-age suppression so a completed
//! generation's terminal is never swallowed by a later mint (doc 05
//! §4.3 invariant 4: exactly one terminal per completed generation).
//!
//! The point of no return is the step-3 flush: a second seek that
//! fails *resolution* (unresolvable target, unseekable source)
//! leaves the older in-flight seek untouched — it still completes
//! and still reports. A second seek that fails *after* the flush
//! (a reanchor refusal) has already invalidated the older seek's
//! pipeline content, so the slot clears and only the new
//! generation's `SeekFailed` is emitted.
//!
//! # Paused seeks — the decode carve-out
//!
//! [`Pipeline::tick`] only decodes in `Buffering`/`Playing`, and the
//! wire table has no `paused → buffering` edge (doc 05 §3.3) — so a
//! seek accepted while `Paused` could never run its step-5 pre-roll.
//! **[PROPOSAL: Lead decision 3.HARD-AUDIT D-a]** — while an
//! `InFlightSeek` exists, the pipeline's decode slice runs even in
//! `Paused`: the wire-visible state stays `paused` (the row is a
//! self-transition), the landing frame may present, and the clock
//! stays frozen until the completion report re-anchors it. Flagged
//! for revisit at 3.C1 (state-machine completeness); the honest
//! alternatives there are a Lead-owned wire edge or an
//! engine-internal transient seeking sub-state.
//!
//! # What is deliberately v0 here
//!
//! - `Chapter` targets refuse typed (the engine has no chapter source
//!   until the doc 03 §4.12 timed-event model lands; hosts resolve
//!   chapters to `Absolute`).
//! - The buffering episode of a Playing-seek still runs through the
//!   watermark machinery (no fast-start seek heuristic).
//! - `FirstFrame` stays once-per-session (2.C2's landed semantics);
//!   whether it re-fires per seek is card 3.B4's event-completeness
//!   question.
//! - The renderer-mailbox and subtitle-active-set clears of doc 03
//!   §6 step 3 have no v0 counterparts (no mailbox, no subtitles);
//!   the [`FrameConsumer`](crate::pipeline::FrameConsumer) seam is
//!   immediate and generation-checked at the sink.

use commands::envelope::RequestId;
use media_clock::{NANO_BASE, PlaybackClock};
use media_types::time::{Stamp, TimeSpan, Timestamp};
use media_types::{EngineError, GenerationId, Nanoseconds};

use crate::pipeline::Pipeline;

/// One seek request, engine-side (doc 05 §2.4 `SeekTarget`, the
/// variants the engine can resolve; `Chapter` refuses typed until the
/// chapter source exists).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekTarget {
    /// µs-faithful absolute position in nanoseconds. Clamped to
    /// `[0, duration]` when the duration is known — clamping is
    /// truth, not failure (doc 05 §2.4).
    Absolute {
        /// Target position (ns from media start; may be negative —
        /// clamped to 0).
        position_ns: i64,
    },
    /// Signed delta from the current position (ns), then clamped.
    Relative {
        /// Negative rewinds.
        delta_ns: i64,
    },
    /// `0.0..=1.0` of the duration; requires a known duration
    /// (unknown → typed out-of-range, doc 05 §2.4).
    Percent {
        /// Fraction of the duration (clamped to `0.0..=1.0`).
        fraction: f32,
    },
    /// Chapter start; the engine has no chapter source in v0 and
    /// refuses typed (hosts resolve chapters to `Absolute`).
    Chapter {
        /// 0-based chapter index.
        index: u32,
    },
}

/// A resolved target: an absolute nanosecond position, or the typed
/// refusal that resolution produced.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedSeek {
    /// The clamped absolute position to hand the demuxer.
    Position {
        /// Absolute media position (ns).
        ns: i64,
    },
    /// Resolution refused (percent without a duration, chapter
    /// without a source).
    Failure(SeekFailure),
}

/// Why a seek failed (the [`EngineEventKind::SeekFailed`] payload;
/// doc 03 §6 `SeekFailed{reason, position}`).
///
/// `Clone`/`PartialEq` are manual: the [`EngineError`] carriers are
/// neither (the [`EngineEventKind::Fatal`] precedent — events are
/// data, not error-chain carriers).
#[derive(Debug)]
pub enum SeekFailure {
    /// The source reports no seek capability (doc 03 §6 step 1:
    /// `SeekSupport::None`) — never reported as success.
    NotSeekable,
    /// The target could not be resolved: percent with an unknown
    /// duration, or a chapter with no chapter source (doc 05 §2.4
    /// `out-of-range`).
    OutOfRange,
    /// The demuxer could not position the stream (out of range,
    /// too-slow scan, corruption) — contract D4 applies: the session
    /// is still at its old position and playable.
    Demux(EngineError),
    /// The clock re-anchor failed (an unrepresentable position).
    Clock(EngineError),
}

impl Clone for SeekFailure {
    fn clone(&self) -> SeekFailure {
        match self {
            SeekFailure::NotSeekable => SeekFailure::NotSeekable,
            SeekFailure::OutOfRange => SeekFailure::OutOfRange,
            // Rebuilt without the boxed `source` chain (the event
            // convention): stage, kind and context code carry the
            // wire-visible truth.
            SeekFailure::Demux(error) => SeekFailure::Demux(shallow(error)),
            SeekFailure::Clock(error) => SeekFailure::Clock(shallow(error)),
        }
    }
}

impl PartialEq for SeekFailure {
    fn eq(&self, other: &SeekFailure) -> bool {
        match (self, other) {
            (SeekFailure::NotSeekable, SeekFailure::NotSeekable)
            | (SeekFailure::OutOfRange, SeekFailure::OutOfRange) => true,
            (SeekFailure::Demux(a), SeekFailure::Demux(b))
            | (SeekFailure::Clock(a), SeekFailure::Clock(b)) => {
                a.stage == b.stage && a.kind == b.kind && a.context.code == b.context.code
            }
            _ => false,
        }
    }
}

/// Rebuild an [`EngineError`] without its boxed source chain.
fn shallow(error: &EngineError) -> EngineError {
    EngineError::new(
        error.stage,
        error.kind.clone(),
        media_types::error::ErrorContext {
            code: error.context.code.clone(),
        },
    )
}

/// The seek failure for an unseekable source (doc 03 §6 step 1),
/// minted engine-side: the refusal is the *engine's* policy fact,
/// not the demuxer's error.
#[must_use]
pub fn not_seekable() -> SeekFailure {
    SeekFailure::NotSeekable
}

/// Resolve a target against the duration and the current position
/// (doc 05 §2.4: absolute/relative clamp to `[0, duration]` when the
/// duration is known; the demuxer's own range check is the backstop
/// when it is not; percent without a duration is typed
/// out-of-range).
pub(crate) fn resolve_target(
    target: SeekTarget,
    duration_ns: Option<i64>,
    position_ns: i64,
) -> ResolvedSeek {
    let clamp = |ns: i64| -> i64 {
        match duration_ns {
            Some(duration) => ns.clamp(0, duration),
            None => ns.max(0),
        }
    };
    match target {
        SeekTarget::Absolute { position_ns } => ResolvedSeek::Position {
            ns: clamp(position_ns),
        },
        SeekTarget::Relative { delta_ns } => ResolvedSeek::Position {
            ns: clamp(position_ns.saturating_add(delta_ns)),
        },
        SeekTarget::Percent { fraction } => match duration_ns {
            Some(duration) => ResolvedSeek::Position {
                ns: clamp((f64::from(fraction.clamp(0.0, 1.0)) * duration as f64).round() as i64),
            },
            None => ResolvedSeek::Failure(SeekFailure::OutOfRange),
        },
        SeekTarget::Chapter { .. } => {
            // No chapter source exists in the engine until the doc 03
            // §4.12 timed-event model lands; the honest answer is the
            // typed refusal, never a silent zero.
            ResolvedSeek::Failure(SeekFailure::OutOfRange)
        }
    }
}

/// The demuxer-facing target span for an absolute nanosecond
/// position.
pub(crate) fn target_span(ns: i64) -> TimeSpan {
    TimeSpan {
        start: Stamp::Present(Timestamp::new(ns, NANO_BASE)),
        end: Stamp::Missing,
    }
}

/// A landing span's start, in nanoseconds (`None` when the demuxer
/// reported no present stamp — a contract violation, surfaced by the
/// caller as a typed failure rather than guessed).
pub(crate) fn span_start_ns(span: &TimeSpan) -> Option<i64> {
    let ts = span.start.timestamp().ok()?;
    match ts.as_nanos() {
        Ok(ns) => Some(nanos_i64(ns)),
        Err(_) => None,
    }
}

/// Lossless-in-practice [`Nanoseconds`] → `i64`: media positions are
/// non-negative and far below `i64::MAX` (≈ 292 years); a theoretical
/// `u64` overflow saturates rather than wraps — never a fabricated
/// position.
pub(crate) fn nanos_i64(ns: Nanoseconds) -> i64 {
    i64::try_from(ns.0).unwrap_or(i64::MAX)
}

/// The seek a session is currently executing (steps 5–7): what was
/// asked, where the demuxer landed, and which generation owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InFlightSeek {
    /// The resolved, clamped target (ns).
    pub target_ns: i64,
    /// The demuxer's actual landing (ns) — the reported truth, not
    /// the request.
    pub landing_ns: i64,
    /// The generation the seek runs under (the mint at accept).
    pub generation: GenerationId,
    /// The request that caused the seek, when attributable (doc 05
    /// §3.1 `origin`).
    pub origin: Option<RequestId>,
}

/// What [`pipeline_seek`] produced: an in-flight seek to continue on
/// the tick path, or the typed failure that ended the flow at step 3
/// or 4 (D4 — the session stays at its old position).
#[derive(Debug, Default)]
pub(crate) struct SeekOutcome {
    /// The recorded in-flight seek (step 4 succeeded).
    pub inflight: Option<InFlightSeek>,
    /// The terminal failure (steps 3/4 failed).
    pub failure: Option<SeekFailure>,
    /// Frames dropped as stale during the seek flush (doc 03 §5
    /// checking point 5 — the decoder-output stale purge).
    pub stale_frames_flushed: u32,
}

/// Steps 3–4 against the live pipeline: flush for `generation`, then
/// re-anchor the demuxer at `target_ns`. Pure with respect to the
/// session — the caller records/emits from the outcome.
pub(crate) fn pipeline_seek(
    pipeline: &mut Pipeline,
    generation: GenerationId,
    target_ns: i64,
    flush_budget: u32,
) -> SeekOutcome {
    let mut outcome = SeekOutcome::default();
    // Step 3 — the flush. A decoder flush/configure failure is a
    // decode-stage refusal of the new generation: the seek fails
    // honestly and the pipeline limps on (the next decode error
    // surfaces through the normal fatal path). The frames the flush
    // already purged are still gone — count them.
    if let Err(error) = pipeline.seek_flush(generation, flush_budget) {
        outcome.stale_frames_flushed = pipeline.take_seek_flush_stale();
        outcome.failure = Some(SeekFailure::Demux(error));
        return outcome;
    }
    outcome.stale_frames_flushed = pipeline.take_seek_flush_stale();
    // Step 4 — the re-anchor (D4: the demuxer commits only on
    // success).
    match pipeline.demuxer_reanchor(target_span(target_ns)) {
        Ok(span) => match span_start_ns(&span) {
            Some(landing_ns) => {
                outcome.inflight = Some(InFlightSeek {
                    target_ns,
                    landing_ns,
                    generation,
                    origin: None,
                });
            }
            None => {
                // The demuxer's own trait contract promises a present
                // landing stamp; a violation is surfaced typed, never
                // guessed around.
                outcome.failure = Some(SeekFailure::Demux(landing_missing()));
            }
        },
        Err(error) => {
            outcome.failure = Some(SeekFailure::Demux(error));
        }
    }
    outcome
}

/// The typed stand-in for a demuxer that re-anchored successfully but
/// reported no landing stamp (a trait-contract violation).
fn landing_missing() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Demux,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new("engine.seek.landing-missing"),
    )
}

/// Steps 6–7 at completion: re-anchor the clock at the truth (the
/// first audio frame's own timestamp when audio exists, else the
/// landing) and answer the `Seeked{requested, actual}` payload.
///
/// # Errors
/// [`SeekFailure::Clock`] when the clock cannot represent the
/// position (the clock is unchanged; the caller reports the failure —
/// never a fake success).
pub(crate) fn complete_seek(
    clock: &media_clock::MediaClock,
    inflight: InFlightSeek,
    audio_anchor_ns: Option<i64>,
) -> Result<i64, SeekFailure> {
    // Step 6 — A_adjusted: anchor on the first audio frame's truth,
    // not the request (doc 03 §6).
    let actual = audio_anchor_ns.unwrap_or(inflight.landing_ns);
    if let Err(error) = clock.reanchor(Timestamp::new(actual, NANO_BASE)) {
        // The media-types prescribed mapping: TimeError → Clock-stage
        // Corrupt{Timing}.
        return Err(SeekFailure::Clock(EngineError::from(error)));
    }
    clock.set_generation(inflight.generation);
    // Step 7 — the report payload.
    Ok(actual)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_clamps_to_the_duration_when_known() {
        let resolved = resolve_target(
            SeekTarget::Absolute {
                position_ns: 90_000_000_000,
            },
            Some(2_000_000_000),
            1_000_000_000,
        );
        assert_eq!(resolved, ResolvedSeek::Position { ns: 2_000_000_000 });
        // Negative clamps to zero.
        let resolved = resolve_target(
            SeekTarget::Absolute { position_ns: -5 },
            Some(2_000_000_000),
            1_000_000_000,
        );
        assert_eq!(
            resolved,
            ResolvedSeek::Position { ns: 0 },
            "clamping is truth, not failure"
        );
        // Unknown duration: no upper clamp (the demuxer range check
        // is the backstop), negative still floors at zero.
        let resolved = resolve_target(
            SeekTarget::Absolute { position_ns: -5 },
            None,
            1_000_000_000,
        );
        assert_eq!(resolved, ResolvedSeek::Position { ns: 0 });
    }

    #[test]
    fn relative_resolves_against_the_position_then_clamps() {
        let resolved = resolve_target(
            SeekTarget::Relative { delta_ns: -1_500 },
            Some(2_000),
            1_000,
        );
        assert_eq!(
            resolved,
            ResolvedSeek::Position { ns: 0 },
            "rewinding past the start clamps to zero"
        );
        let resolved = resolve_target(SeekTarget::Relative { delta_ns: 500 }, Some(2_000), 1_000);
        assert_eq!(resolved, ResolvedSeek::Position { ns: 1_500 });
    }

    #[test]
    fn percent_needs_a_known_duration() {
        let resolved = resolve_target(SeekTarget::Percent { fraction: 0.5 }, None, 0);
        assert_eq!(
            resolved,
            ResolvedSeek::Failure(SeekFailure::OutOfRange),
            "percent with unknown duration is the typed out-of-range"
        );
        let resolved = resolve_target(
            SeekTarget::Percent { fraction: 0.5 },
            Some(2_000_000_000),
            0,
        );
        assert_eq!(resolved, ResolvedSeek::Position { ns: 1_000_000_000 });
        // Out-of-range fractions clamp (truth), including negatives.
        let resolved = resolve_target(SeekTarget::Percent { fraction: 1.5 }, Some(2_000), 0);
        assert_eq!(resolved, ResolvedSeek::Position { ns: 2_000 });
        let resolved = resolve_target(SeekTarget::Percent { fraction: -0.5 }, Some(2_000), 0);
        assert_eq!(resolved, ResolvedSeek::Position { ns: 0 });
    }

    #[test]
    fn chapter_refuses_typed_until_a_chapter_source_exists() {
        let resolved = resolve_target(SeekTarget::Chapter { index: 2 }, Some(2_000), 0);
        assert_eq!(resolved, ResolvedSeek::Failure(SeekFailure::OutOfRange));
    }

    #[test]
    fn target_span_and_back_is_lossless_in_nano_base() {
        let span = target_span(1_234_567);
        assert_eq!(span_start_ns(&span), Some(1_234_567));
        // A missing stamp answers None (surfaced, never guessed).
        assert_eq!(span_start_ns(&TimeSpan::UNKNOWN), None);
    }

    #[test]
    fn failure_clones_and_compares_without_source_chains() {
        let a = SeekFailure::Demux(landing_missing());
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(SeekFailure::NotSeekable, SeekFailure::OutOfRange);
    }
}
