//! The scheduling policy vocabulary (card 2.C4; doc 03 §4.10 b/e):
//! [`DropPolicy`], [`SchedulerCfg`] and the pure per-frame classifier
//! [`FrameDecision`].
//!
//! Everything here is pure arithmetic over the media-types time
//! vocabulary — no clock reads, no state, no I/O — so the doc 03
//! §4.10(f) requirement ("assert exact present/drop/repeat decisions
//! for every policy parameter combination, exhaustive over small
//! grids") is testable directly against these types.
//!
//! # Time base
//!
//! All positions this module speaks are [`Timestamp`]s on the clock's
//! canonical nanosecond base ([`media_clock::NANO_BASE`]) — the same
//! base [`media_clock::PlaybackClock::now`] returns. Producers
//! normalize stream-base PTS with [`Timestamp::convert`] (checked)
//! before scheduling; this module never guesses a base.
//!
//! # Decision order (doc 03 §4.10 e, binding)
//!
//! 1. **Late first**: `pts < now − threshold` → [`FrameDecision::SkipLate`]
//!    (strict `<`; `pts == now − threshold` is *on* the boundary and
//!    presents). A frame can be both late and due — late wins.
//! 2. **Due**: `pts ≤ now + target_lead` → [`FrameDecision::Present`]
//!    (inclusive `≤`, per "the largest `pts ≤ clock + target_lead`").
//! 3. Otherwise the frame is early → [`FrameDecision::WaitUntil`] at
//!    `pts − target_lead`, the earliest clock position at which it
//!    becomes due.

use std::num::NonZeroUsize;

use media_clock::NANO_BASE;
use media_types::time::{Nanoseconds, Timestamp};

/// The late-frame drop policy (doc 03 §4.10 b).
///
/// "Audio is never dropped for video's sake" (doc 03 §4.10 e) — this
/// policy governs *video* frames only, always.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPolicy {
    /// Never drop for lateness: late frames are still presented (in
    /// order) when they are the next due frame. Degenerate but
    /// spec'd — the diagnostic configuration ("show every frame").
    Never,
    /// Drop frames whose `pts < now − threshold`; drop, count, advance
    /// (doc 03 §4.10 e). The threshold default is one frame duration
    /// `[PROPOSAL per doc 03 §4.10 e]`; configurable, MP §5.
    Late {
        /// How far past `now` a frame's PTS may be before it is
        /// dropped. Zero is legal (drop anything not yet on time).
        threshold: Nanoseconds,
    },
    /// Automatic. v0 resolves to [`DropPolicy::Late`] with the doc's
    /// default threshold — one nominal frame duration — and therefore
    /// requires [`SchedulerCfg::frame_duration`]
    /// `[PROPOSAL: the adaptive refinement (drop only when a newer
    /// frame is ready, widen under sustained pressure) is Phase-3
    /// hardening with MP §11 drop-rate measurements; the resolved
    /// default is the documented starting point, not a tuning claim]`.
    Auto,
}

impl DropPolicy {
    /// The effective late threshold for this policy given the nominal
    /// frame duration: `None` means "never drop for lateness".
    ///
    /// `Auto` resolves to the frame duration when known — the doc 03
    /// §4.10 e default — and to "never" when unknown (an unmeasured
    /// guess would violate MP §8's never-fabricate rule; the
    /// constructor [`SchedulerCfg::new`] rejects that combination
    /// instead).
    pub fn late_threshold(self, frame_duration: Option<Nanoseconds>) -> Option<Nanoseconds> {
        match self {
            DropPolicy::Never => None,
            DropPolicy::Late { threshold } => Some(threshold),
            DropPolicy::Auto => frame_duration,
        }
    }
}

/// The per-frame scheduling decision (doc 01 §2.2 interface sketch:
/// "present, wait, or skip-late"; doc 03 §4.10 e invariants).
///
/// Pure: a function of (PTS, clock position, [`SchedulerCfg`]) only.
/// The payload-free adaptation of the sketch's
/// `Present(VideoFrame)`/`SkipLate { pts }` is deliberate — the
/// window scheduler ([`crate::PresentationScheduler`]) owns the frames
/// and enriches decisions with them; a pure classifier stays
/// grid-testable (doc 03 §4.10 f).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDecision {
    /// The frame's slot is now: `pts ≤ now + target_lead` and it is not
    /// beyond the late threshold.
    Present,
    /// The frame is early. `due_at` is the clock position at which it
    /// becomes due (`pts − target_lead`; may be in the past only when
    /// the lead overflows the position range — then it saturates).
    WaitUntil {
        /// Earliest clock position at which this frame is due.
        due_at: Timestamp,
    },
    /// The frame is beyond the late threshold: `pts < now − threshold`
    /// (doc 03 §4.10 e). Drops are counted, never silent.
    SkipLate,
}

/// Invalid scheduler configuration (typed, per the media-types error
/// discipline: failure is data, never a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCfgError {
    /// [`DropPolicy::Auto`] needs a nominal frame duration to resolve
    /// its threshold; none was configured.
    AutoWithoutFrameDuration,
    /// A configured frame duration of zero is corrupt timing (a frame
    /// with no extent can never be scheduled).
    ZeroFrameDuration,
}

impl std::fmt::Display for SchedulerCfgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerCfgError::AutoWithoutFrameDuration => write!(
                f,
                "drop policy `auto` needs a nominal frame duration to resolve its late threshold"
            ),
            SchedulerCfgError::ZeroFrameDuration => {
                write!(f, "frame duration must be known-positive, got zero")
            }
        }
    }
}

impl std::error::Error for SchedulerCfgError {}

/// The presentation-scheduler configuration (doc 03 §4.10 b sketch,
/// extended with the fields v0 needs to implement its invariants).
///
/// All durations are [`Nanoseconds`] (unsigned — a negative lead or
/// threshold is unrepresentable, so the sign class of error the sketch
/// allowed cannot exist).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerCfg {
    /// The late-frame drop policy. See [`DropPolicy`].
    pub policy: DropPolicy,
    /// How many consecutive ticks may repeat the last frame before the
    /// scheduler falls back to background (doc 03 §4.10 e "up to
    /// `max_repeat`, then background"). Zero is legal: never repeat.
    pub max_repeat: usize,
    /// The vsync lead: a frame is due when `pts ≤ now + target_lead`
    /// (doc 03 §4.10 e). The doc's proposed default is half a frame
    /// duration `[PROPOSAL per doc 03 §4.10 e]`.
    pub target_lead: Nanoseconds,
    /// The nominal frame duration, when the track's rate is known.
    /// Feeds [`DropPolicy::Auto`]'s threshold and the missing-PTS
    /// inference (media-types `TimeError::Missing` contract).
    pub frame_duration: Option<Nanoseconds>,
    /// The reorder-window bound: how many frames may sit in the
    /// scheduler window awaiting their slots. Bounded by construction —
    /// an unbounded window is unconstructible (MP §8, doc 03 §4.10 c
    /// "a small reorder window").
    pub max_pending: NonZeroUsize,
}

/// Default reorder-window capacity `[PROPOSAL: doc 03 §4.10 c says "a
/// small reorder window" without a number; 16 covers a 4-frame B-pyramid
/// at 4× decode burst, the worst v0 fixture shape — revisit with Phase-3
/// measurements]`.
pub const DEFAULT_MAX_PENDING: usize = 16;

/// Default repeat budget `[PROPOSAL: doc 03 §4.10 e specifies the
/// repeat-then-background *shape* but no count; 3 holds one frame
/// through a brief decoder stall (~2 extra vsyncs at 30 fps) before
/// clearing — a product.toml knob per MP §5, not a fixed truth]`.
pub const DEFAULT_MAX_REPEAT: usize = 3;

impl SchedulerCfg {
    /// The fully explicit, checked constructor.
    ///
    /// # Errors
    /// [`SchedulerCfgError::AutoWithoutFrameDuration`] when the policy
    /// is [`DropPolicy::Auto`] and no frame duration is configured;
    /// [`SchedulerCfgError::ZeroFrameDuration`] when a configured
    /// duration is zero.
    pub fn new(
        policy: DropPolicy,
        max_repeat: usize,
        target_lead: Nanoseconds,
        frame_duration: Option<Nanoseconds>,
        max_pending: NonZeroUsize,
    ) -> Result<SchedulerCfg, SchedulerCfgError> {
        if matches!(policy, DropPolicy::Auto) && frame_duration.is_none() {
            return Err(SchedulerCfgError::AutoWithoutFrameDuration);
        }
        if frame_duration == Some(Nanoseconds(0)) {
            return Err(SchedulerCfgError::ZeroFrameDuration);
        }
        Ok(SchedulerCfg {
            policy,
            max_repeat,
            target_lead,
            frame_duration,
            max_pending,
        })
    }

    /// The doc's default configuration for a track with a known
    /// nominal frame duration: [`DropPolicy::Late`] with a one-frame
    /// threshold, half-frame lead, [`DEFAULT_MAX_REPEAT`] and
    /// [`DEFAULT_MAX_PENDING`] (doc 03 §4.10 e — both defaults there
    /// are `[PROPOSAL]`-tagged; the constants carry their own tags).
    ///
    /// # Errors
    /// [`SchedulerCfgError::ZeroFrameDuration`] when `duration` is zero.
    pub fn for_frame_duration(duration: Nanoseconds) -> Result<SchedulerCfg, SchedulerCfgError> {
        SchedulerCfg::new(
            DropPolicy::Late {
                threshold: duration,
            },
            DEFAULT_MAX_REPEAT,
            Nanoseconds(duration.0 / 2),
            Some(duration),
            NonZeroUsize::new(DEFAULT_MAX_PENDING).unwrap_or_else(|| unreachable!()),
        )
    }

    /// The resolved late threshold of this configuration (see
    /// [`DropPolicy::late_threshold`]).
    pub fn late_threshold(&self) -> Option<Nanoseconds> {
        self.policy.late_threshold(self.frame_duration)
    }

    /// Classify one frame against one clock position (doc 03 §4.10 e,
    /// decision order: late → due → wait). Both positions must be on
    /// the clock's nanosecond base — the base [`media_clock::
    /// PlaybackClock::now`] returns and [`crate::PresentationScheduler`]
    /// normalizes PTS into.
    ///
    /// Boundary semantics (tested exhaustively over small grids):
    /// `pts == now` presents; `pts == now + target_lead` presents
    /// (inclusive); `pts == now − threshold` presents (strict late
    /// comparison); one nanosecond past either bound flips the
    /// decision.
    #[must_use = "a scheduling decision ignored is a frame presented at the wrong time or dropped silently"]
    pub fn decide(&self, pts: Timestamp, now: Timestamp) -> FrameDecision {
        let pts_ns = i128::from(pts.ticks);
        let now_ns = i128::from(now.ticks);
        if let Some(late_bound) = self
            .late_threshold()
            .map(|threshold| now_ns - i128::from(threshold.0))
            && pts_ns < late_bound
        {
            return FrameDecision::SkipLate;
        }
        if pts_ns <= now_ns + i128::from(self.target_lead.0) {
            FrameDecision::Present
        } else {
            FrameDecision::WaitUntil {
                due_at: self.due_at(pts),
            }
        }
    }

    /// The clock position at which `pts` becomes due (`pts −
    /// target_lead`), saturating at the [`Timestamp`] tick range —
    /// a wait target beyond ±292 years is indistinguishable from its
    /// clamp for any real driver, and saturation (documented, tested)
    /// beats wrapping.
    #[must_use = "the wait target is what a timer-driven driver sleeps until"]
    pub fn due_at(&self, pts: Timestamp) -> Timestamp {
        let due = i128::from(pts.ticks) - i128::from(self.target_lead.0);
        let clamped = due.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
        Timestamp::new(clamped as i64, NANO_BASE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    fn ns(ticks: i64) -> Timestamp {
        Timestamp::new(ticks, NANO_BASE)
    }

    fn cfg(threshold: Option<u64>, lead: u64) -> SchedulerCfg {
        let policy = match threshold {
            Some(t) => DropPolicy::Late {
                threshold: Nanoseconds(t),
            },
            None => DropPolicy::Never,
        };
        SchedulerCfg::new(
            policy,
            DEFAULT_MAX_REPEAT,
            Nanoseconds(lead),
            Some(Nanoseconds(10)),
            NonZeroUsize::new(4).must(),
        )
        .must()
    }

    // ---- decide(): exact decisions over a small exhaustive grid ------

    #[test]
    fn grid_late_policy_decisions_are_exact() {
        // threshold 40, lead 5; now 1000. Offsets sweep far-late through
        // the boundaries to far-early (doc 03 §4.10 f "exhaustive over
        // small grids").
        let c = cfg(Some(40), 5);
        let now = ns(1000);
        let cases: [(i64, FrameDecision); 12] = [
            (0, FrameDecision::SkipLate),                          // far late
            (900, FrameDecision::SkipLate),                        // just past late bound − 1
            (959, FrameDecision::SkipLate),                        // late bound − 1
            (960, FrameDecision::Present), // pts == now − threshold: ON the bound, presents
            (961, FrameDecision::Present), // just inside
            (1000, FrameDecision::Present), // pts == now
            (1005, FrameDecision::Present), // pts == now + lead: inclusive
            (1006, FrameDecision::WaitUntil { due_at: ns(1001) }), // one past due bound
            (1010, FrameDecision::WaitUntil { due_at: ns(1005) }),
            (2000, FrameDecision::WaitUntil { due_at: ns(1995) }),
            (i64::MIN, FrameDecision::SkipLate), // extreme: still classified, no overflow
            (
                i64::MAX,
                FrameDecision::WaitUntil {
                    due_at: ns(i64::MAX - 5),
                },
            ),
        ];
        for (pts, want) in cases {
            assert_eq!(c.decide(ns(pts), now), want, "pts {pts} ns vs now 1000");
        }
    }

    #[test]
    fn grid_never_policy_never_skips() {
        let c = cfg(None, 0);
        let now = ns(500);
        for pts in [i64::MIN, -1, 0, 499, 500, 501, i64::MAX] {
            let got = c.decide(ns(pts), now);
            assert_ne!(got, FrameDecision::SkipLate, "never drops pts {pts}");
        }
        // With zero lead, due is exact: pts == now presents, past waits.
        assert_eq!(c.decide(ns(500), now), FrameDecision::Present);
        assert_eq!(
            c.decide(ns(501), now),
            FrameDecision::WaitUntil { due_at: ns(501) }
        );
    }

    #[test]
    fn zero_threshold_drops_anything_not_yet_on_time() {
        let c = cfg(Some(0), 0);
        let now = ns(100);
        assert_eq!(c.decide(ns(99), now), FrameDecision::SkipLate);
        assert_eq!(c.decide(ns(100), now), FrameDecision::Present);
        assert_eq!(
            c.decide(ns(101), now),
            FrameDecision::WaitUntil { due_at: ns(101) }
        );
    }

    #[test]
    fn drops_are_monotone_in_lateness() {
        // Doc 03 §4.10 f property: "drops are monotone in lateness" —
        // a frame dropped at `now` is dropped at every later `now` with
        // the same config. Bounded-exhaustive over the grid (a property
        // framework is a doc 04 candidate; the grid is deterministic).
        let c = cfg(Some(30), 4);
        for pts in [0i64, 50, 99, 100, 101, 250] {
            let mut first_drop_now: Option<i64> = None;
            for now in (0i64..400).step_by(7) {
                let dropped = c.decide(ns(pts), ns(now)) == FrameDecision::SkipLate;
                match (dropped, first_drop_now) {
                    (true, None) => first_drop_now = Some(now),
                    (false, Some(drop_now)) => {
                        unreachable!("pts {pts} dropped at {drop_now} but presented at {now}")
                    }
                    _ => {}
                }
            }
            // pts 0, 50, 99 are inside the threshold's eventual reach:
            // each must have dropped at some grid position.
            if pts < 100 {
                first_drop_now.must();
            }
        }
    }

    #[test]
    fn waits_are_monotone_in_earliness() {
        // The dual: a frame waiting at `now` was waiting (or presenting
        // at the exact due tick) at every earlier `now`.
        let c = cfg(Some(10), 8);
        for pts in [200i64, 350] {
            for now in (0i64..400).step_by(3) {
                if let FrameDecision::WaitUntil { due_at } = c.decide(ns(pts), ns(now)) {
                    assert_eq!(due_at, ns(pts - 8));
                }
            }
        }
    }

    #[test]
    fn extreme_positions_never_overflow() {
        // i128 arithmetic throughout: every extreme pairing classifies
        // without panicking or wrapping (the saturation discipline of
        // media-clock positions, applied to due targets).
        let c = cfg(Some(u64::MAX), u64::MAX);
        for now in [i64::MIN, -1, 0, 1, i64::MAX] {
            for pts in [i64::MIN, -1, 0, i64::MAX] {
                let _ = c.decide(ns(pts), ns(now));
            }
        }
        // A lead of u64::MAX makes every representable position due.
        assert_eq!(c.decide(ns(i64::MAX), ns(0)), FrameDecision::Present);
        // A due target below the i64 tick range saturates at it (the
        // documented clamp; reachable only through `due_at` directly —
        // `decide`'s wait branch always lands above `now`).
        assert_eq!(c.due_at(ns(i64::MIN)), ns(i64::MIN));
        // A representable target subtracts exactly.
        let small = SchedulerCfg::new(
            DropPolicy::Never,
            0,
            Nanoseconds(7),
            None,
            NonZeroUsize::new(1).must(),
        )
        .must();
        assert_eq!(small.due_at(ns(100)), ns(93));
    }

    // ---- DropPolicy resolution ---------------------------------------

    #[test]
    fn policy_threshold_resolution() {
        let dur = Some(Nanoseconds(33));
        assert_eq!(DropPolicy::Never.late_threshold(dur), None);
        assert_eq!(
            DropPolicy::Late {
                threshold: Nanoseconds(7)
            }
            .late_threshold(None),
            Some(Nanoseconds(7))
        );
        // Auto: the doc's default — one frame duration — when known.
        assert_eq!(DropPolicy::Auto.late_threshold(dur), Some(Nanoseconds(33)));
        // Unknown duration: no guess (never fabricate).
        assert_eq!(DropPolicy::Auto.late_threshold(None), None);
    }

    // ---- configuration validation ------------------------------------

    #[test]
    fn auto_without_duration_is_a_typed_config_error() {
        let err = assert_err(
            SchedulerCfg::new(
                DropPolicy::Auto,
                0,
                Nanoseconds(0),
                None,
                NonZeroUsize::new(1).must(),
            ),
            "auto needs a duration",
        );
        assert_eq!(err, SchedulerCfgError::AutoWithoutFrameDuration);
        let shown = err.to_string();
        assert!(shown.contains("auto"), "{shown}");
        assert!(shown.contains("frame duration"), "{shown}");
    }

    #[test]
    fn zero_duration_is_a_typed_config_error() {
        let err = assert_err(
            SchedulerCfg::new(
                DropPolicy::Late {
                    threshold: Nanoseconds(1),
                },
                0,
                Nanoseconds(0),
                Some(Nanoseconds(0)),
                NonZeroUsize::new(1).must(),
            ),
            "zero duration rejected",
        );
        assert_eq!(err, SchedulerCfgError::ZeroFrameDuration);
        assert!(err.to_string().contains("zero"), "{}", err);
    }

    #[test]
    fn for_frame_duration_applies_the_doc_defaults() {
        // 30 fps → 33_333_334 ns frames [half-frame lead floors].
        let c = SchedulerCfg::for_frame_duration(Nanoseconds(33_333_334)).must();
        assert_eq!(
            c.policy,
            DropPolicy::Late {
                threshold: Nanoseconds(33_333_334)
            }
        );
        assert_eq!(c.target_lead, Nanoseconds(16_666_667));
        assert_eq!(c.max_repeat, DEFAULT_MAX_REPEAT);
        assert_eq!(c.max_pending.get(), DEFAULT_MAX_PENDING);
        assert_eq!(c.frame_duration, Some(Nanoseconds(33_333_334)));
    }
}
