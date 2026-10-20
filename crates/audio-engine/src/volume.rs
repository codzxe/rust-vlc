//! Per-track gain: linear or dB (card 2.C3; doc 03 §4.8 `MixPoint`'s
//! `gain_db` vocabulary).
//!
//! The v0 volume is a plain scalar multiply in the f32 mixer domain —
//! no curves, no compression, no balance split (pan is `PanLaw`-shaped
//! work for the filter-chain card, doc 03 §4.8 b). Linear gain is
//! non-negative and finite; dB gain is finite and converts as
//! `10^(db/20)` (the 20·log10 amplitude convention).

use media_types::EngineError;

use crate::error;

/// A usable gain value.
///
/// Constructors validate; the stored value is always finite and
/// (linearly) non-negative, so the DSP can multiply without defensive
/// checks (NaN never enters the mix — a NaN sample stays a NaN sample,
/// which is *data*, but a NaN *gain* would poison every output and is
/// rejected at the control boundary).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume {
    linear: f32,
}

impl Volume {
    /// Unity gain — the exact identity on samples (multiplying by 1.0
    /// is bit-preserving in IEEE 754).
    pub const UNITY: Volume = Volume { linear: 1.0 };

    /// Silence — the exact zero (multiplying by 0.0 preserves the sign
    /// of a NaN payload but zeroes everything finite).
    pub const MUTE: Volume = Volume { linear: 0.0 };

    /// A linear gain. `0.0` mutes; `1.0` is unity; values above 1.0
    /// amplify (clipping is the stage-D limiter's business,
    /// doc 03 §4.8 e).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.gain_not_finite` / `audio.gain_negative`: gains must be
    /// finite and non-negative (phase inversion is a filter, not a
    /// volume).
    pub fn from_linear(linear: f32) -> Result<Volume, EngineError> {
        if !linear.is_finite() {
            return Err(error::bad_gain("audio.gain_not_finite"));
        }
        if linear < 0.0 {
            return Err(error::bad_gain("audio.gain_negative"));
        }
        Ok(Volume { linear })
    }

    /// A dB gain (amplitude convention: `linear = 10^(db/20)`; 0 dB is
    /// unity, −6 dB ≈ 0.50119, +6 dB ≈ 1.99526).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.gain_db_not_finite`: the dB value must be finite
    /// (callers that mean silence say [`Volume::MUTE`]; keeping the
    /// domain finite keeps the conversion total and deterministic).
    pub fn from_db(db: f32) -> Result<Volume, EngineError> {
        if !db.is_finite() {
            return Err(error::bad_gain("audio.gain_db_not_finite"));
        }
        // 10^(db/20) via std: powf is deterministic per IEEE 754 on
        // every host for the same input (exp10 is not in std).
        Ok(Volume {
            linear: 10.0_f32.powf(db / 20.0),
        })
    }

    /// The linear gain (finite, non-negative by construction).
    pub fn linear(self) -> f32 {
        self.linear
    }

    /// The dB rendering (amplitude convention, `20·log10(linear)`).
    /// `MUTE` renders as [`f32::NEG_INFINITY`] — the honest zero
    /// on a log scale (documented, not clamped).
    pub fn db(self) -> f32 {
        20.0 * self.linear.log10()
    }

    /// Apply to one sample (the inner-loop primitive: one multiply).
    pub fn apply(self, sample: f32) -> f32 {
        sample * self.linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn unity_is_bit_preserving() {
        // Multiplying by exactly 1.0 preserves every f32 bit — the
        // golden-buffer identity the AC relies on.
        let values = [
            0.0f32,
            -0.0,
            1.0,
            -1.0,
            0.5,
            16384.0 / 32768.0,
            f32::MIN,
            f32::MAX,
        ];
        for v in values {
            assert_eq!(
                Volume::UNITY.apply(v).to_bits(),
                v.to_bits(),
                "unity changed {v}"
            );
        }
        assert_eq!(Volume::UNITY.linear(), 1.0);
        assert_eq!(Volume::UNITY.db(), 0.0);
    }

    #[test]
    fn mute_zeroes_everything_finite() {
        for v in [0.0f32, 1.0, -1.0, 1e30] {
            assert_eq!(Volume::MUTE.apply(v), 0.0);
        }
        assert!(Volume::MUTE.db().is_infinite() && Volume::MUTE.db() < 0.0);
    }

    #[test]
    fn db_construction_matches_the_amplitude_convention() {
        let minus6 = Volume::from_db(-6.0).must();
        // 10^(-6/20) = 0.501187...
        assert!(
            (minus6.linear() - 0.501_187).abs() < 1e-4,
            "{}",
            minus6.linear()
        );
        assert!((minus6.db() - (-6.0)).abs() < 1e-3);
        let plus6 = Volume::from_db(6.0).must();
        assert!(
            (plus6.linear() - 1.995_262).abs() < 1e-4,
            "{}",
            plus6.linear()
        );
        // Round trip within float noise.
        let v = Volume::from_db(-23.5).must();
        assert!((v.db() - (-23.5)).abs() < 1e-2);
    }

    #[test]
    fn linear_construction_rejects_bad_values() {
        let codes = [
            (f32::NAN, "audio.gain_not_finite"),
            (f32::INFINITY, "audio.gain_not_finite"),
            (-1.0, "audio.gain_negative"),
        ];
        for (value, code) in codes {
            let err = match Volume::from_linear(value) {
                Err(e) => e,
                Ok(v) => unreachable!("accepted {value} as {}", v.linear()),
            };
            assert_eq!(err.context.code, code, "{value}");
        }
        let err = Volume::from_db(f32::NAN).err().must();
        assert_eq!(err.context.code, "audio.gain_db_not_finite");
    }

    #[test]
    fn apply_is_a_pure_multiply() {
        let v = Volume::from_linear(0.5).must();
        assert_eq!(v.apply(0.75), 0.375);
        assert_eq!(v.apply(-0.75), -0.375);
        // NaN payload survives (data, not poison-from-the-gain).
        assert!(v.apply(f32::NAN).is_nan());
    }
}
