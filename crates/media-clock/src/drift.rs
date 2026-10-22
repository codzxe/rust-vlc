//! Bounded drift correction (card 2.C1; doc 03 §4.7 e).
//!
//! Doc 03 §4.7(e) assigns drift to two absorbers, neither of which is
//! the clock itself: video presentation error is absorbed by the video
//! scheduler (§4.10), audio-side drift (device clock vs media rate) by
//! a bounded micro-resampler in audio-engine. What §4.7(e) fixes as the
//! *invariant* — and what this module therefore provides — is the
//! **limiter**: "correction rate limited to ±1000 ppm
//! `[PROPOSAL — final number from Phase 3 measurements; the limiter
//! itself is the invariant, not the value]`".
//!
//! [`DriftCorrection`] is that bounded rate adjustment: it is computed
//! from a measured divergence between a master and a follower
//! ([`DriftCorrection::for_divergence`]) and composes onto a playback
//! [`Rate`] exactly (rational arithmetic, no float drift — doc 01
//! §3.1). Nothing here resamples audio or schedules video; the
//! consumers (audio-engine, video-engine) apply the correction, and
//! they can rely on it never exceeding the bound.
//!
//! **Consumers (card 3.B1 — audit M12 closed):** audio-engine's
//! `MicroResampler` consumes a stored [`DriftCorrection`] as its
//! resample ratio (`out = in × (1e6 + ppm)/1e6`, crates/audio-engine/
//! src/resample.rs), and its `DriftGovernor` *derives* one per window
//! via [`DriftCorrection::for_divergence`] over the measured ring-fill
//! error (crates/audio-engine/src/governor.rs). The clock itself is
//! never drift-corrected (doc 03 §4.7 e: the two absorbers are the
//! video scheduler and the micro-resampler — never `set_rate`).

use media_types::time::{Rate, TimeBase, TimeError};

/// The maximum drift-correction magnitude, in parts per million of the
/// nominal rate (doc 03 §4.7 e). **The limit is the invariant; the
/// ±1000 ppm value is `[PROPOSAL]`** — the final number comes from
/// Phase 3 measurements. Changing it is a one-line, one-test change.
pub const MAX_CORRECTION_PPM: i64 = 1000;

/// `ppm` scaling factor: 1 ppm = 1e-6 of the nominal rate.
const PPM_SCALE: i128 = 1_000_000;

/// A bounded playback-rate adjustment for drift correction — always
/// within `±`[`MAX_CORRECTION_PPM`] by construction (the limiter).
///
/// The sign convention: a *positive* correction speeds the follower
/// up (it lags the master), a *negative* one slows it down (it leads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DriftCorrection {
    ppm: i64,
}

impl DriftCorrection {
    /// No correction.
    pub const ZERO: DriftCorrection = DriftCorrection { ppm: 0 };

    /// Construct with the limiter applied: values beyond
    /// `±`[`MAX_CORRECTION_PPM`] saturate to the bound. Saturation is
    /// the specified semantics (the correction *never* exceeds the
    /// bound); callers that need to observe the clamp compare their
    /// desired value against [`DriftCorrection::ppm`].
    pub fn new(ppm: i64) -> DriftCorrection {
        DriftCorrection {
            ppm: ppm.clamp(-MAX_CORRECTION_PPM, MAX_CORRECTION_PPM),
        }
    }

    /// The correction in ppm — always within the bound.
    pub fn ppm(self) -> i64 {
        self.ppm
    }

    /// The correction for a measured divergence of
    /// `follower_minus_master_ns` over an `observation_window_ns`
    /// window: `ppm = -(divergence / window) x 1e6`, saturated.
    ///
    /// A follower *behind* the master (negative divergence) gets a
    /// positive (speed-up) correction and vice versa. A zero
    /// observation window carries no information: [`DriftCorrection::ZERO`]
    /// (never a division by zero, never a fabricated maximum).
    pub fn for_divergence(
        follower_minus_master_ns: i128,
        observation_window_ns: u64,
    ) -> DriftCorrection {
        if observation_window_ns == 0 {
            return DriftCorrection::ZERO;
        }
        let window = i128::from(observation_window_ns);
        // desired ppm = -(divergence x 1e6 / window), ties away from
        // zero; every step checked (an i128 extreme saturates to the
        // bound in the correction's needed direction).
        let desired = match follower_minus_master_ns.checked_mul(PPM_SCALE) {
            Some(scaled) => match div_round(scaled, window) {
                Some(rounded) => match rounded.checked_neg() {
                    Some(desired) => {
                        i64::try_from(desired).unwrap_or_else(|_| extreme(follower_minus_master_ns))
                    }
                    None => extreme(follower_minus_master_ns),
                },
                None => extreme(follower_minus_master_ns),
            },
            None => extreme(follower_minus_master_ns),
        };
        DriftCorrection::new(desired)
    }

    /// Compose this correction onto a playback rate:
    /// `rate x (1 + ppm / 1e6)`, exact rational arithmetic (reduced;
    /// no float drift), checked.
    ///
    /// # Errors
    /// [`TimeError::Overflow`] when the composed rate's components do
    /// not fit `u32` (the taxonomy's unrepresentable-conversion
    /// variant; context fields are diagnostic — see the module docs of
    /// [`crate::mapping`]).
    pub fn apply(self, rate: Rate) -> Result<Rate, TimeError> {
        if rate.num == 0 || rate.den == 0 {
            return Err(rate_overflow(rate));
        }
        // factor = (1e6 + ppm) / 1e6, always positive: |ppm| <= 1000.
        let factor_num = PPM_SCALE + i128::from(self.ppm);
        let factor_den = PPM_SCALE;
        let num = i128::from(rate.num).checked_mul(factor_num);
        let den = i128::from(rate.den).checked_mul(factor_den);
        let (Some(num), Some(den)) = (num, den) else {
            return Err(rate_overflow(rate));
        };
        let g = gcd128(num, den);
        let (num, den) = (num / g, den / g);
        let num = u32::try_from(num).map_err(|_| rate_overflow(rate))?;
        let den = u32::try_from(den).map_err(|_| rate_overflow(rate))?;
        // Cannot fail for checked non-zero components, but no unwrap:
        // a Rate::new error maps to the same typed overflow.
        Rate::new(num, den).map_err(|_| rate_overflow(rate))
    }
}

impl std::fmt::Display for DriftCorrection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:+} ppm", self.ppm)
    }
}

/// The saturation extreme in the correction's needed direction.
fn extreme(signed: i128) -> i64 {
    if signed < 0 { i64::MAX } else { i64::MIN }
}

/// Divide with rounding, ties away from zero; `None` on a degenerate
/// denominator (the `media-types` discipline).
fn div_round(numer: i128, denom: i128) -> Option<i128> {
    if denom <= 0 {
        return None;
    }
    let (q, r) = (numer / denom, numer % denom);
    let twice_r = r.checked_mul(2)?;
    if twice_r >= denom {
        q.checked_add(1)
    } else if twice_r <= -denom {
        q.checked_sub(1)
    } else {
        Some(q)
    }
}

fn gcd128(mut a: i128, mut b: i128) -> i128 {
    debug_assert!(a > 0 && b > 0);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The taxonomy-consistent overflow for unrepresentable rate
/// composition (diagnostic context fields, [`crate::mapping`] docs).
fn rate_overflow(rate: Rate) -> TimeError {
    TimeError::Overflow {
        from: TimeBase { num: 1, den: 1 },
        to: TimeBase { num: 1, den: 1 },
        ticks: i64::from(rate.num),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(num: u32, den: u32) -> Rate {
        Rate::new(num, den).unwrap_or_else(|e| unreachable!("{e}"))
    }

    #[test]
    fn new_applies_the_limiter() {
        assert_eq!(DriftCorrection::new(500).ppm(), 500);
        assert_eq!(DriftCorrection::new(-500).ppm(), -500);
        assert_eq!(DriftCorrection::new(1000).ppm(), 1000);
        // Beyond the bound: saturated, never exceeding it.
        assert_eq!(DriftCorrection::new(100_000).ppm(), MAX_CORRECTION_PPM);
        assert_eq!(DriftCorrection::new(-100_000).ppm(), -MAX_CORRECTION_PPM);
        assert_eq!(DriftCorrection::new(i64::MAX).ppm(), MAX_CORRECTION_PPM);
        assert_eq!(DriftCorrection::new(i64::MIN).ppm(), -MAX_CORRECTION_PPM);
    }

    #[test]
    fn divergence_signs_speed_the_right_way() {
        // Follower 1 ms behind over a 10 s window: +100 ppm (speed up).
        let lagging = DriftCorrection::for_divergence(-1_000_000, 10_000_000_000);
        assert_eq!(lagging.ppm(), 100);
        // Follower 1 ms ahead over 10 s: -100 ppm (slow down).
        let leading = DriftCorrection::for_divergence(1_000_000, 10_000_000_000);
        assert_eq!(leading.ppm(), -100);
        // Zero divergence: zero correction.
        assert_eq!(DriftCorrection::for_divergence(0, 10_000_000_000).ppm(), 0);
        // Zero window: no information, no correction, no panic.
        assert_eq!(DriftCorrection::for_divergence(-1_000_000, 0).ppm(), 0);
    }

    #[test]
    fn divergence_saturates_at_the_bound() {
        // 100 ms of divergence over a 1 s window wants -100000 ppm.
        let huge = DriftCorrection::for_divergence(100_000_000, 1_000_000_000);
        assert_eq!(huge.ppm(), -MAX_CORRECTION_PPM);
        // i128-extreme divergences saturate too (checked math).
        let extreme_lag = DriftCorrection::for_divergence(i128::MIN + 1, 1);
        assert_eq!(extreme_lag.ppm(), MAX_CORRECTION_PPM);
        let extreme_lead = DriftCorrection::for_divergence(i128::MAX, 1);
        assert_eq!(extreme_lead.ppm(), -MAX_CORRECTION_PPM);
    }

    #[test]
    fn apply_composes_exactly() {
        // 1x + 100 ppm = 1000100/1000000 = 10001/10000 exactly.
        let corrected = DriftCorrection::new(100)
            .apply(Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(corrected, rate(10001, 10000));
        // 1x - 1000 ppm = 999000/1000000 = 999/1000.
        let corrected = DriftCorrection::new(-1000)
            .apply(Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(corrected, rate(999, 1000));
        // Composition preserves base rates: 5/4 + 0 ppm stays 5/4.
        assert_eq!(
            DriftCorrection::ZERO
                .apply(rate(5, 4))
                .unwrap_or(rate(1, 1)),
            rate(5, 4)
        );
        // 5/4 + 1000 ppm = 5 x 1001000 / (4 x 1000000) = 5005/4000.
        let corrected = DriftCorrection::new(1000)
            .apply(rate(5, 4))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(corrected, rate(5005, 4000));
    }

    #[test]
    fn apply_stays_within_the_bound() {
        // The invariant: for ANY divergence, the composed rate is
        // within (1 +/- MAX_CORRECTION_PPM/1e6) of the base.
        for divergence_ns in [
            -1i128,
            -1_000_000_000,
            -1_000_000_000_000,
            i128::MIN + 1,
            1,
            1_000_000_000,
            1_000_000_000_000,
            i128::MAX,
        ] {
            for window in [1u64, 1_000_000, 10_000_000_000] {
                let correction = DriftCorrection::for_divergence(divergence_ns, window);
                let corrected = correction
                    .apply(Rate::ONE)
                    .unwrap_or_else(|e| unreachable!("{e}"));
                let (num, den) = corrected.as_parts();
                let (num, den) = (i128::from(num), i128::from(den));
                let lower = PPM_SCALE - i128::from(MAX_CORRECTION_PPM);
                let upper = PPM_SCALE + i128::from(MAX_CORRECTION_PPM);
                assert!(
                    num * PPM_SCALE >= den * lower && num * PPM_SCALE <= den * upper,
                    "corrected {corrected} escapes the +-{} ppm bound",
                    MAX_CORRECTION_PPM
                );
            }
        }
    }

    #[test]
    fn apply_overflows_typed_on_unrepresentable_rates() {
        // A base rate whose corrected numerator cannot fit u32.
        let huge = Rate {
            num: u32::MAX,
            den: 1,
        };
        assert!(DriftCorrection::new(1000).apply(huge).is_err());
        // Degenerate rates are rejected typed (no panic, no division).
        assert!(
            DriftCorrection::new(1)
                .apply(Rate { num: 0, den: 1 })
                .is_err()
        );
        assert!(
            DriftCorrection::new(1)
                .apply(Rate { num: 1, den: 0 })
                .is_err()
        );
    }

    #[test]
    fn display_is_signed_and_bounded() {
        assert_eq!(DriftCorrection::new(100).to_string(), "+100 ppm");
        assert_eq!(DriftCorrection::new(-100).to_string(), "-100 ppm");
        assert_eq!(DriftCorrection::ZERO.to_string(), "+0 ppm");
    }
}
