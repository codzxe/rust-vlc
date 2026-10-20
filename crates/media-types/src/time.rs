//! Timing types (doc 01 §3.2).
//!
//! The core rule: **a timestamp value and its absence are different
//! types, and conversion failure is data, not a panic** (MP §8). All
//! arithmetic is checked; overflow returns [`TimeError::Overflow`] with
//! full context, never wrap-around.

use crate::error::{EngineError, ErrorContext, ErrorKind, PipelineStage};
use crate::ids::GenerationId;

/// Ticks per second = `num / den` (e.g. `1/90000` for MPEG timing).
///
/// Constructed through [`TimeBase::new`], which rejects zero components
/// and normalizes to lowest terms, so two bases describing the same rate
/// compare equal (MPEG's `1/90000` and a demuxer's `2/180000` unify).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeBase {
    pub num: u32,
    pub den: u32,
}

impl TimeBase {
    /// A valid, reduced time base.
    ///
    /// # Errors
    /// [`TimeError::Missing`] is *not* used for invalid construction (a
    /// missing timestamp is runtime data, not a constructor bug);
    /// construction errors are [`EngineError`]s with kind
    /// [`Unsupported`](ErrorKind::Unsupported) and stage `Demux` when
    /// either component is zero.
    pub fn new(num: u32, den: u32) -> Result<TimeBase, EngineError> {
        if num == 0 || den == 0 {
            return Err(EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("timebase.zero_component"),
            ));
        }
        let g = gcd(num, den);
        Ok(TimeBase {
            num: num / g,
            den: den / g,
        })
    }

    /// The ticks-per-second rate as an exact rational `(num, den)`,
    /// already reduced.
    pub fn as_parts(self) -> (u32, u32) {
        (self.num, self.den)
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl std::fmt::Display for TimeBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl std::str::FromStr for TimeBase {
    type Err = EngineError;

    /// Parse `"num/den"` (e.g. `"1/90000"`). Enables the serialization
    /// round-trip required by doc 01 §2.2(e) without pulling serde into
    /// this std-only crate `[PROPOSAL]`.
    fn from_str(s: &str) -> Result<TimeBase, EngineError> {
        let bad = || {
            EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("timebase.parse"),
            )
        };
        let (num, den) = s.split_once('/').ok_or_else(bad)?;
        let num: u32 = num.trim().parse().map_err(|_| bad())?;
        let den: u32 = den.trim().parse().map_err(|_| bad())?;
        TimeBase::new(num, den)
    }
}

/// Signed ticks in a [`TimeBase`]. Negative ticks exist in the wild
/// `[VERIFY: which containers — doc 04 evidence]`, so ticks are `i64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp {
    pub ticks: i64,
    pub base: TimeBase,
}

impl Timestamp {
    /// Construct a timestamp (the base is validated by construction).
    ///
    /// # Errors
    /// Propagates [`TimeBase::new`]'s error for zero components.
    pub fn new(ticks: i64, base: TimeBase) -> Timestamp {
        Timestamp { ticks, base }
    }

    /// Build from parts, validating the base.
    ///
    /// # Errors
    /// See [`TimeBase::new`].
    pub fn from_parts(ticks: i64, num: u32, den: u32) -> Result<Timestamp, EngineError> {
        Ok(Timestamp::new(ticks, TimeBase::new(num, den)?))
    }

    /// Checked conversion to another base. Overflow is an explicit error,
    /// never wrap-around; inexact ratios round to the nearest tick (ties
    /// away from zero) — the demuxer-standard rescale.
    ///
    /// # Errors
    /// [`TimeError::Overflow`] when the converted tick count cannot be
    /// represented as `i64` (all intermediate math is `i128`).
    pub fn convert(&self, to: &TimeBase) -> Result<Timestamp, TimeError> {
        // value_seconds = ticks * from.num / from.den
        // ticks_to       = value_seconds * to.den / to.num
        //               = ticks * from.num * to.den / (from.den * to.num)
        let from = &self.base;
        let numer = i128::from(self.ticks) * i128::from(from.num) * i128::from(to.den);
        let denom = i128::from(from.den) * i128::from(to.num);
        let converted = div_round_i128(numer, denom).ok_or(TimeError::Overflow {
            from: *from,
            to: *to,
            ticks: self.ticks,
        })?;
        let ticks = i64::try_from(converted).map_err(|_| TimeError::Overflow {
            from: *from,
            to: *to,
            ticks: self.ticks,
        })?;
        Ok(Timestamp::new(ticks, *to))
    }

    /// Checked conversion to nanoseconds.
    ///
    /// # Errors
    /// [`TimeError::Overflow`] when the value exceeds `u64` nanoseconds
    /// (≈ 584 years — generous, but streams are adversarial).
    pub fn as_nanos(&self) -> Result<Nanoseconds, TimeError> {
        let nanos = div_round_i128(
            i128::from(self.ticks) * i128::from(self.base.num) * 1_000_000_000i128,
            i128::from(self.base.den),
        )
        .ok_or(TimeError::Overflow {
            from: self.base,
            to: NANOS_BASE,
            ticks: self.ticks,
        })?;
        let nanos = u64::try_from(nanos).map_err(|_| TimeError::Overflow {
            from: self.base,
            to: NANOS_BASE,
            ticks: self.ticks,
        })?;
        Ok(Nanoseconds(nanos))
    }
}

/// The `1/1_000_000_000` base used by [`Nanoseconds`] (internal only).
const NANOS_BASE: TimeBase = TimeBase {
    num: 1,
    den: 1_000_000_000,
};

/// Divide with rounding (ties away from zero). `None` when the result
/// cannot fit `i128` after rounding (only possible for denominators < 1,
/// which cannot occur — kept for exhaustive checkedness).
fn div_round_i128(numer: i128, denom: i128) -> Option<i128> {
    debug_assert!(denom > 0);
    if denom <= 0 {
        return None;
    }
    let (q, r) = (numer / denom, numer % denom);
    let twice_r = r.checked_mul(2)?;
    let adjusted = if twice_r >= denom {
        q.checked_add(1)?
    } else if twice_r <= -denom {
        q.checked_sub(1)?
    } else {
        q
    };
    Some(adjusted)
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Ticks + base — the unambiguous, round-trippable rendering
        // (seconds renderings can be lossy and invite parse-back bugs).
        write!(f, "{}@{}", self.ticks, self.base)
    }
}

/// Time-domain errors (doc 01 §3.2). Exactly the three spec variants —
/// overflow, discontinuity, missing — each carrying full context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeError {
    /// A conversion that cannot be represented. The caller decides:
    /// rescale via nanoseconds, clamp with an event, or fail.
    Overflow {
        from: TimeBase,
        to: TimeBase,
        ticks: i64,
    },
    /// Arithmetic across a discontinuity without an explicit clock reset.
    /// The reset is a generation-scoped operation (doc 01 §3.2).
    Discontinuity {
        expected: Option<Timestamp>,
        found: Timestamp,
        generation: GenerationId,
    },
    /// A required timestamp was absent. Propagates; the presentation
    /// scheduler treats it as "infer from previous frame + duration,
    /// else hold" — the inference is an observable event, never silent
    /// invention (MP §8: never fabricate).
    Missing,
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeError::Overflow { from, to, ticks } => {
                write!(
                    f,
                    "time conversion overflow: {ticks} ticks of {from} cannot be represented in {to}"
                )
            }
            TimeError::Discontinuity {
                expected,
                found,
                generation,
            } => match expected {
                Some(expected) => write!(
                    f,
                    "time discontinuity: expected {expected}, found {found} (generation {})",
                    generation.counter()
                ),
                None => write!(
                    f,
                    "time discontinuity: no expected timestamp, found {found} (generation {})",
                    generation.counter()
                ),
            },
            TimeError::Missing => f.write_str("timestamp missing"),
        }
    }
}

impl std::error::Error for TimeError {}

impl From<TimeError> for EngineError {
    /// Time errors become stage-tagged engine errors at the `Clock`
    /// stage (the clock owns conversion policy; demuxer/decoder errors
    /// that carry a `TimeError` source route through their own stages).
    fn from(err: TimeError) -> EngineError {
        let code = match &err {
            TimeError::Overflow { .. } => "time.overflow",
            TimeError::Discontinuity { .. } => "time.discontinuity",
            TimeError::Missing => "time.missing",
        };
        EngineError::new(
            PipelineStage::Clock,
            ErrorKind::Corrupt {
                detail: crate::error::CorruptDetail::Timing,
            },
            ErrorContext::new(code),
        )
        .with_source(err)
    }
}

/// Packet/frame stamping: present or absent — no `-1` sentinel (MP §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stamp {
    Present(Timestamp),
    Missing,
}

impl Stamp {
    /// Unwrap the timestamp, mapping absence to [`TimeError::Missing`].
    ///
    /// # Errors
    /// [`TimeError::Missing`] when the stamp is absent.
    pub fn timestamp(&self) -> Result<Timestamp, TimeError> {
        match self {
            Stamp::Present(ts) => Ok(*ts),
            Stamp::Missing => Err(TimeError::Missing),
        }
    }

    /// `Some(true/false)` when present, `None` when missing — for
    /// comparison-style call sites that treat absence as "unknown".
    pub fn map_timestamp<U>(&self, f: impl FnOnce(Timestamp) -> U) -> Option<U> {
        match self {
            Stamp::Present(ts) => Some(f(*ts)),
            Stamp::Missing => None,
        }
    }
}

/// A start/end span in stamps. Either end may be missing (live streams).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSpan {
    pub start: Stamp,
    pub end: Stamp,
}

impl TimeSpan {
    /// A span with both ends missing (fully unknown extent).
    pub const UNKNOWN: TimeSpan = TimeSpan {
        start: Stamp::Missing,
        end: Stamp::Missing,
    };
}

/// Track timing summary reported with every [`crate::ids::Track`]:
/// span, time base, known gaps (doc 01 §3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackTiming {
    /// The track's extent (either end may be `Missing`).
    pub span: TimeSpan,
    /// The base all track timestamps share.
    pub base: TimeBase,
    /// Container-declared gaps (edits, splices) in track time.
    pub gaps: Vec<TimeSpan>,
}

/// Nanoseconds — the engine's interchange time unit for wall-clock-style
/// durations (doc 01 §3.1). Newtype, not a bare `u64`, so a duration can
/// never be confused with a byte count or a tick count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nanoseconds(pub u64);

impl Nanoseconds {
    /// Exact seconds when the value divides evenly, else `None`.
    pub fn as_secs_exact(self) -> Option<u64> {
        self.0
            .is_multiple_of(1_000_000_000)
            .then_some(self.0 / 1_000_000_000)
    }
}

/// Unknown durations are common in streams — an explicit variant, never
/// `0` and never `-1` (MP §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Duration {
    Known(Nanoseconds),
    Unknown,
}

impl Duration {
    /// Unwrap known durations; `Duration::Unknown` maps to
    /// [`TimeError::Missing`].
    ///
    /// # Errors
    /// [`TimeError::Missing`] when unknown.
    pub fn nanos(&self) -> Result<Nanoseconds, TimeError> {
        match self {
            Duration::Known(n) => Ok(*n),
            Duration::Unknown => Err(TimeError::Missing),
        }
    }

    /// Format as `H:MM:SS.mmm` for display; `Unknown` renders as `"--:--"`
    /// (a deliberately non-numeric string — it can never be parsed back
    /// into a number, so no sentinel-value confusion is possible).
    pub fn display(&self) -> String {
        match self {
            Duration::Unknown => "--:--".to_string(),
            Duration::Known(n) => {
                let total_ms = n.0 / 1_000_000;
                let (h, m, s, ms) = (
                    total_ms / 3_600_000,
                    (total_ms / 60_000) % 60,
                    (total_ms / 1_000) % 60,
                    total_ms % 1_000,
                );
                format!("{h}:{m:02}:{s:02}.{ms:03}")
            }
        }
    }
}

/// Rational playback rate. `1×` = `1/1`; `1.25×` = `5/4` — exact, no
/// float drift (doc 01 §3.1).
///
/// Negative rates (reverse playback) are out of v1 scope `[PROPOSAL]`;
/// the feature matrix tracks the gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rate {
    pub num: u32,
    pub den: u32,
}

impl Rate {
    /// Normal (forward) playback.
    pub const ONE: Rate = Rate { num: 1, den: 1 };

    /// A valid, reduced positive rate.
    ///
    /// # Errors
    /// [`EngineError`] with kind
    /// [`Unsupported`](ErrorKind::Unsupported) and stage `Control` when
    /// either component is zero (a zero rate is pause, not a rate).
    pub fn new(num: u32, den: u32) -> Result<Rate, EngineError> {
        if num == 0 || den == 0 {
            return Err(EngineError::new(
                PipelineStage::Control,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("rate.zero_component"),
            ));
        }
        let g = gcd(num, den);
        Ok(Rate {
            num: num / g,
            den: den / g,
        })
    }

    /// The rate as an exact rational `(num, den)`, already reduced.
    pub fn as_parts(self) -> (u32, u32) {
        (self.num, self.den)
    }
}

impl std::fmt::Display for Rate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl std::str::FromStr for Rate {
    type Err = EngineError;

    /// Parse `"num/den"` (e.g. `"5/4"`); pairs with `Display` for the
    /// doc 01 §2.2(e) serialization round-trip.
    fn from_str(s: &str) -> Result<Rate, EngineError> {
        let bad = || {
            EngineError::new(
                PipelineStage::Control,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("rate.parse"),
            )
        };
        let (num, den) = s.split_once('/').ok_or_else(bad)?;
        let num: u32 = num.trim().parse().map_err(|_| bad())?;
        let den: u32 = den.trim().parse().map_err(|_| bad())?;
        Rate::new(num, den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tiny deterministic LCG — "property tests" without a dependency
    // (std-only crate, doc 01 §2.2(c)); every case is reproducible from
    // the seed printed in the failure message.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 11
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    // ---- TimeBase -----------------------------------------------------------

    #[test]
    fn timebase_rejects_zero_components() {
        assert!(TimeBase::new(0, 90000).is_err());
        assert!(TimeBase::new(1, 0).is_err());
    }

    #[test]
    fn timebase_reduces_to_lowest_terms() {
        let tb = TimeBase::new(2, 180_000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            tb,
            TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}"))
        );
        assert_eq!(tb.to_string(), "1/90000");
    }

    #[test]
    fn timebase_display_parse_roundtrip() {
        for (num, den) in [(1u32, 90_000u32), (1, 1000), (1001, 30000), (3, 7)] {
            let tb = TimeBase::new(num, den).unwrap_or_else(|e| unreachable!("{e}"));
            let s = tb.to_string();
            let back: TimeBase = s.parse().unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(tb, back, "roundtrip failed for {s}");
        }
    }

    // ---- Timestamp::convert -------------------------------------------------

    #[test]
    fn convert_is_exact_when_bases_agree() {
        let ts = Timestamp::from_parts(1234, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}"));
        let same = ts
            .convert(&TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(same.ticks, 1234);
    }

    #[test]
    fn convert_rescales_mpeg_to_milliseconds() {
        // 90000 ticks at 1/90000 = 1 second = 1000 ticks at 1/1000.
        let ts = Timestamp::from_parts(90_000, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}"));
        let ms = ts
            .convert(&TimeBase::new(1, 1000).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(ms.ticks, 1000);
        assert_eq!(
            ms.base,
            TimeBase::new(1, 1000).unwrap_or_else(|e| unreachable!("{e}"))
        );
    }

    #[test]
    fn convert_rounds_ties_away_from_zero() {
        // A true tie: from=1/2, to=1/1, ticks=1 => 1*1*1/2 = 0.5 -> 1.
        let ts = Timestamp::from_parts(1, 1, 2).unwrap_or_else(|e| unreachable!("{e}"));
        let conv = ts
            .convert(&TimeBase::new(1, 1).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(conv.ticks, 1, "0.5 rounds away from zero");

        let neg = Timestamp::from_parts(-1, 1, 2).unwrap_or_else(|e| unreachable!("{e}"));
        let conv = neg
            .convert(&TimeBase::new(1, 1).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(conv.ticks, -1, "-0.5 rounds away from zero");
    }

    #[test]
    fn convert_overflow_is_an_explicit_error() {
        // i64::MAX ticks of 1/90000 into nanoseconds (1/1e9) overflows i64.
        let ts = Timestamp::from_parts(i64::MAX, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}"));
        match ts.convert(&NANOS_BASE) {
            Err(TimeError::Overflow { from, to, ticks }) => {
                assert_eq!(ticks, i64::MAX);
                assert_eq!(
                    from,
                    TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}"))
                );
                assert_eq!(to, NANOS_BASE);
            }
            other => unreachable!("expected overflow, got {other:?}"),
        }
    }

    #[test]
    fn as_nanos_matches_reference_for_common_bases() {
        // Deterministic sweep instead of floats: for a grid of bases and
        // ticks, the i128 reference and as_nanos agree within rounding.
        let mut rng = Lcg(0xC0FFEE);
        for _ in 0..2000 {
            let den = 1 + rng.below(100_000);
            let num = 1 + rng.below(4);
            let base =
                TimeBase::new(num as u32, den as u32).unwrap_or_else(|e| unreachable!("{e}"));
            // Keep the product safely inside u64 nanoseconds:
            // ticks * num * 1e9 <= u64::MAX.
            let max_ticks = u64::MAX / 1_000_000_000 / num * den;
            let ticks = rng.below(max_ticks) as i64;
            let ts = Timestamp::new(ticks, base);
            let nanos = ts.as_nanos().unwrap_or_else(|e| unreachable!("{e}"));
            // reference: exact rational in i128
            let exact =
                i128::from(ticks) * i128::from(base.num) * 1_000_000_000i128 / i128::from(base.den);
            let got = i128::from(nanos.0);
            let tolerance = 1i128; // rounding to nearest: |error| <= 0.5 tick scaled
            assert!(
                (got - exact).abs() <= tolerance,
                "nanos drifted: base {base}, ticks {ticks}, exact {exact}, got {got}"
            );
        }
    }

    #[test]
    fn convert_roundtrip_is_identity_when_exact() {
        let mut rng = Lcg(0xBEEF);
        for _ in 0..2000 {
            let b1 = TimeBase::new(1, 1 + rng.below(90_000) as u32)
                .unwrap_or_else(|e| unreachable!("{e}"));
            let b2 = TimeBase::new(1, 1 + rng.below(1000) as u32)
                .unwrap_or_else(|e| unreachable!("{e}"));
            let ticks = rng.below(1 << 32) as i64;
            let ts = Timestamp::new(ticks, b1);
            if let Ok(mid) = ts.convert(&b2)
                && let Ok(back) = mid.convert(&b1)
            {
                // Each direction rounds to nearest: total absolute
                // error <= 0.5*(a+b) seconds expressed in a-ticks:
                //   err_ticks <= 0.5 * (1 + (a.den*b.num)/(a.num*b.den))
                // plus 2 ticks of slack for the two exact divisions.
                let ratio = (u128::from(b1.den) * u128::from(b2.num))
                    / (u128::from(b1.num) * u128::from(b2.den));
                let allowed = (ratio / 2) as i64 + 2;
                assert!(
                    (back.ticks - ticks).abs() <= allowed,
                    "roundtrip drifted: {ticks} {b1} -> {b2} -> {} {b1} (allowed {allowed})",
                    back.ticks
                );
            }
        }
    }

    #[test]
    fn nanos_overflow_errors_rather_than_wrapping() {
        let ts = Timestamp::from_parts(i64::MAX, 1, 1).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(ts.as_nanos().is_err());
    }

    // ---- Stamp / Duration -----------------------------------------------------

    #[test]
    fn stamp_missing_maps_to_time_error_missing() {
        assert_eq!(Stamp::Missing.timestamp().err(), Some(TimeError::Missing));
        let ts = Timestamp::from_parts(5, 1, 1000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            Stamp::Present(ts)
                .timestamp()
                .unwrap_or_else(|e| unreachable!("{e}")),
            ts
        );
        assert_eq!(Stamp::Missing.map_timestamp(|t| t.ticks), None);
        assert_eq!(Stamp::Present(ts).map_timestamp(|t| t.ticks), Some(5));
    }

    #[test]
    fn duration_unknown_is_never_zero_or_neg_one() {
        assert_eq!(Duration::Unknown.display(), "--:--");
        assert_eq!(Duration::Unknown.nanos().err(), Some(TimeError::Missing));
        let d = Duration::Known(Nanoseconds(3_723_004_000_000));
        assert_eq!(d.display(), "1:02:03.004");
        assert_eq!(
            d.nanos()
                .unwrap_or_else(|e| unreachable!("{e}"))
                .as_secs_exact(),
            None
        );
        assert_eq!(
            Duration::Known(Nanoseconds(2_000_000_000))
                .nanos()
                .unwrap_or_else(|e| unreachable!("{e}"))
                .as_secs_exact(),
            Some(2)
        );
    }

    // ---- Rate -------------------------------------------------------------------

    #[test]
    fn rate_is_exact_and_reduced() {
        let r = Rate::new(10, 8).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(r.as_parts(), (5, 4)); // 1.25x exact
        assert_eq!(Rate::ONE.as_parts(), (1, 1));
        assert!(Rate::new(0, 1).is_err());
        assert!(Rate::new(1, 0).is_err());
    }

    #[test]
    fn rate_display_parse_roundtrip() {
        for (num, den) in [(5u32, 4u32), (1, 1), (3, 2), (29, 7)] {
            let r = Rate::new(num, den).unwrap_or_else(|e| unreachable!("{e}"));
            let back: Rate = r
                .to_string()
                .parse()
                .unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(r, back);
        }
    }

    // ---- TimeError display + EngineError conversion -----------------------------

    #[test]
    fn time_error_displays_with_context() {
        let e = TimeError::Overflow {
            from: TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}")),
            to: TimeBase::new(1, 1).unwrap_or_else(|e| unreachable!("{e}")),
            ticks: -7,
        };
        let msg = e.to_string();
        assert!(msg.contains("overflow"), "{msg}");
        assert!(msg.contains("-7"), "{msg}");
        let engine: EngineError = e.into();
        assert_eq!(engine.stage, PipelineStage::Clock);
        assert_eq!(engine.context.code, "time.overflow");
        assert!(matches!(
            engine.kind,
            ErrorKind::Corrupt {
                detail: crate::error::CorruptDetail::Timing
            }
        ));
    }
}
