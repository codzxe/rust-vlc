//! Three-band biquad equalizer (issue #33 M2 — the `afeat-equalizer`
//! row's engine core: low/mid/high shelf + preamp, pure Rust).
//!
//! RBJ audio-EQ-cookbook biquads (Robert Bristow-Johnson's public
//! formulas — a published specification, clean-room compatible).
//! Each band is a normalized direct-form-1 biquad with per-channel
//! state; [`Eq3Band`] composes preamp → low shelf → mid peak → high
//! shelf and plugs into [`EffectChain`](crate::EffectChain) as an
//! [`AudioEffect`].
//!
//! # Zero-dB identity
//!
//! At 0 dB every band's normalized coefficients collapse to the exact
//! identity filter — the output is **bit-identical** to the input
//! (pinned by test; it is what makes a flat EQ free, not merely
//! transparent).
//!
//! # Hot path
//!
//! `process` allocates only on the first quantum (per-channel state
//! warm-up, the pipeline's stage-A capacity discipline) — after that
//! it is five multiplies and four adds per sample per band (D-1).

use media_types::EngineError;

use crate::error;
use crate::volume::Volume;

use super::AudioEffect;

/// Normalized biquad coefficients (RBJ cookbook).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoeffs {
    fn normalize(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> BiquadCoeffs {
        BiquadCoeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// RBJ low shelf: gain `gain_db` below `freq` Hz (Q sets the
    /// corner steepness; 0.707 is the maximally-flat default).
    ///
    /// # Errors
    /// `audio.effects_eq_params` for non-finite inputs or a corner
    /// frequency outside `(0, rate/2)`.
    pub fn low_shelf(rate: u32, freq: f32, gain_db: f32, q: f32) -> Result<BiquadCoeffs, EngineError> {
        check_params(rate, freq, gain_db, q)?;
        let (a, cw, alpha) = precompute(rate, freq, gain_db, q);
        let sqrt_a = a.sqrt();
        Ok(BiquadCoeffs::normalize(
            a * ((a + 1.0) - (a - 1.0) * cw + 2.0 * sqrt_a * alpha),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cw),
            a * ((a + 1.0) - (a - 1.0) * cw - 2.0 * sqrt_a * alpha),
            (a + 1.0) + (a - 1.0) * cw + 2.0 * sqrt_a * alpha,
            -2.0 * ((a - 1.0) + (a + 1.0) * cw),
            (a + 1.0) + (a - 1.0) * cw - 2.0 * sqrt_a * alpha,
        ))
    }

    /// RBJ high shelf: gain `gain_db` above `freq` Hz.
    ///
    /// # Errors
    /// Same as [`low_shelf`](BiquadCoeffs::low_shelf).
    pub fn high_shelf(rate: u32, freq: f32, gain_db: f32, q: f32) -> Result<BiquadCoeffs, EngineError> {
        check_params(rate, freq, gain_db, q)?;
        let (a, cw, alpha) = precompute(rate, freq, gain_db, q);
        let sqrt_a = a.sqrt();
        Ok(BiquadCoeffs::normalize(
            a * ((a + 1.0) + (a - 1.0) * cw + 2.0 * sqrt_a * alpha),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cw),
            a * ((a + 1.0) + (a - 1.0) * cw - 2.0 * sqrt_a * alpha),
            (a + 1.0) - (a - 1.0) * cw + 2.0 * sqrt_a * alpha,
            2.0 * ((a - 1.0) - (a + 1.0) * cw),
            (a + 1.0) - (a - 1.0) * cw - 2.0 * sqrt_a * alpha,
        ))
    }

    /// RBJ peaking EQ: gain `gain_db` at `freq` Hz (Q is the
    /// bandwidth-in-octaves denominator, cookbook `alpha = sin/(2Q)`).
    ///
    /// # Errors
    /// Same as [`low_shelf`](BiquadCoeffs::low_shelf).
    pub fn peaking(rate: u32, freq: f32, gain_db: f32, q: f32) -> Result<BiquadCoeffs, EngineError> {
        check_params(rate, freq, gain_db, q)?;
        let (a, cw, alpha) = precompute(rate, freq, gain_db, q);
        Ok(BiquadCoeffs::normalize(
            1.0 + alpha * a,
            -2.0 * cw,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cw,
            1.0 - alpha / a,
        ))
    }

    /// The identity coefficients (unity transfer at every frequency).
    pub const IDENTITY: BiquadCoeffs = BiquadCoeffs {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };
}

fn check_params(rate: u32, freq: f32, gain_db: f32, q: f32) -> Result<(), EngineError> {
    if !freq.is_finite() || !gain_db.is_finite() || !q.is_finite() || q <= 0.0 {
        return Err(error::unsupported_format("audio.effects_eq_params"));
    }
    if freq <= 0.0 || freq >= (rate as f32) / 2.0 {
        return Err(error::unsupported_format("audio.effects_eq_params"));
    }
    Ok(())
}

fn precompute(rate: u32, freq: f32, gain_db: f32, q: f32) -> (f32, f32, f32) {
    // A = 10^(G/40) — the shelf's amplitude ratio (cookbook).
    let a = 10.0_f32.powf(gain_db / 40.0);
    let w = 2.0 * std::f32::consts::PI * freq / (rate as f32);
    let cw = w.cos();
    let alpha = w.sin() / (2.0 * q);
    (a, cw, alpha)
}

/// One biquad with per-channel direct-form-1 state.
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    /// (z1, z2) per channel — sized on first process (warm-up).
    state: Vec<[f32; 2]>,
}

impl BiquadFilter {
    /// A filter at `coeffs`.
    pub fn new(coeffs: BiquadCoeffs) -> BiquadFilter {
        BiquadFilter {
            coeffs,
            state: Vec::new(),
        }
    }

    /// The current coefficients.
    pub fn coeffs(&self) -> BiquadCoeffs {
        self.coeffs
    }

    fn process(&mut self, planes: &mut [Vec<f32>]) {
        if self.state.len() != planes.len() {
            // First quantum (or channel-count change): warm the state.
            self.state = vec![[0.0; 2]; planes.len()];
        }
        for (ch, plane) in planes.iter_mut().enumerate() {
            let c = self.coeffs;
            let z = &mut self.state[ch];
            for x in plane.iter_mut() {
                let y = c.b0 * *x + z[0];
                z[0] = c.b1 * *x - c.a1 * y + z[1];
                z[1] = c.b2 * *x - c.a2 * y;
                *x = y;
            }
        }
    }
}

/// Three-band EQ: preamp → low shelf → mid peak → high shelf.
#[derive(Debug, Clone)]
pub struct Eq3Band {
    preamp: Volume,
    low: BiquadFilter,
    mid: BiquadFilter,
    high: BiquadFilter,
}

impl Eq3Band {
    /// Build from band gains (dB). Corners: low 250 Hz shelf, mid
    /// 1 kHz peak, high 4 kHz shelf (Q 0.707 shelves / 1.0 peak) —
    /// the design panels' three-knob essentials.
    ///
    /// # Errors
    /// `audio.effects_eq_params` for bad band parameters (see the
    /// biquad constructors).
    pub fn new(rate: u32, low_db: f32, mid_db: f32, high_db: f32, preamp: Volume) -> Result<Eq3Band, EngineError> {
        Ok(Eq3Band {
            preamp,
            low: BiquadFilter::new(BiquadCoeffs::low_shelf(rate, 250.0, low_db, 0.707)?),
            mid: BiquadFilter::new(BiquadCoeffs::peaking(rate, 1000.0, mid_db, 1.0)?),
            high: BiquadFilter::new(BiquadCoeffs::high_shelf(rate, 4000.0, high_db, 0.707)?),
        })
    }

    /// A flat EQ at `rate` (the exact identity — see module docs).
    pub fn flat(rate: u32) -> Result<Eq3Band, EngineError> {
        Eq3Band::new(rate, 0.0, 0.0, 0.0, Volume::UNITY)
    }

    /// The preamp volume.
    pub fn preamp(&self) -> Volume {
        self.preamp
    }
}

impl AudioEffect for Eq3Band {
    fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError> {
        let g = self.preamp.linear();
        if g != 1.0 {
            for plane in planes.iter_mut() {
                for sample in plane.iter_mut() {
                    *sample *= g;
                }
            }
        }
        self.low.process(planes);
        self.mid.process(planes);
        self.high.process(planes);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "eq3"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn sine(n: usize, freq: f32, a: f32) -> Vec<f32> {
        (0..n)
            .map(|i| a * (2.0 * std::f32::consts::PI * freq * i as f32 / RATE as f32).sin())
            .collect()
    }

    /// Peak amplitude of the settled tail (transients discarded).
    fn settled_peak(plane: &[f32]) -> f32 {
        let tail = &plane[plane.len() / 2..];
        tail.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    /// RMS of the settled tail. Phase-independent over integer
    /// periods — the honest amplitude probe for frequencies whose
    /// discrete samples never hit the crest (3 samples/period etc.).
    fn settled_rms(plane: &[f32]) -> f32 {
        let tail = &plane[plane.len() / 2..];
        (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt()
    }

    #[test]
    fn flat_eq_is_bit_exact_identity() {
        let mut eq = Eq3Band::flat(RATE).unwrap();
        let original = sine(4800, 997.0, 0.6); // 997 Hz: prime, hits all bands
        let mut planes = vec![original.clone()];
        eq.process(&mut planes).unwrap();
        assert_eq!(planes[0], original, "0 dB shelves must be bit-exact");
    }

    #[test]
    fn identity_coeffs_are_the_trivial_filter() {
        assert_eq!(BiquadCoeffs::IDENTITY.b0, 1.0);
        assert_eq!(BiquadCoeffs::IDENTITY.b1, 0.0);
    }

    #[test]
    fn low_shelf_boosts_bass_by_the_asked_db() {
        // +6 dB low shelf, corner 250 Hz: a 40 Hz sine (well below the
        // corner) settles at ×10^(6/20) ≈ 1.995.
        let mut eq = Eq3Band::new(RATE, 6.0, 0.0, 0.0, Volume::UNITY).unwrap();
        let mut planes = vec![sine(24_000, 40.0, 0.5)];
        eq.process(&mut planes).unwrap();
        let measured = settled_peak(&planes[0]);
        let expected = 0.5 * 10.0_f32.powf(6.0 / 20.0);
        assert!(
            (measured - expected).abs() / expected < 0.05,
            "measured {measured} vs expected {expected}"
        );
    }

    #[test]
    fn high_shelf_cuts_treble_by_the_asked_db() {
        // −6 dB high shelf, corner 4 kHz: the cookbook transfer
        // function gives |H(16 kHz)| = −5.996 dB (4× the corner is
        // settled to 0.004 dB). A 16 kHz sine at 48 kHz is 3 samples
        // per period — sample-peak never reaches the true crest, so
        // the amplitude is measured as settled RMS over the tail's
        // integer periods (24_000 samples = exact 3-sample periods;
        // phase-independent by construction).
        let mut eq = Eq3Band::new(RATE, 0.0, 0.0, -6.0, Volume::UNITY).unwrap();
        let mut planes = vec![sine(24_000, 16_000.0, 0.5)];
        eq.process(&mut planes).unwrap();
        let input_rms = 0.5 / std::f32::consts::SQRT_2;
        let measured = settled_rms(&planes[0]) / input_rms;
        let expected = 10.0_f32.powf(-6.0 / 20.0);
        assert!(
            (measured - expected).abs() / expected < 0.02,
            "measured {measured} vs expected {expected}"
        );
    }

    #[test]
    fn mid_peak_gains_at_center_frequency() {
        // +6 dB peaking at 1 kHz: a 1 kHz sine settles at ≈ ×1.995
        // (the peaking filter's |H| at w0 is A by construction).
        let mut eq = Eq3Band::new(RATE, 0.0, 6.0, 0.0, Volume::UNITY).unwrap();
        let mut planes = vec![sine(24_000, 1000.0, 0.4)];
        eq.process(&mut planes).unwrap();
        let measured = settled_peak(&planes[0]);
        let expected = 0.4 * 10.0_f32.powf(6.0 / 20.0);
        assert!(
            (measured - expected).abs() / expected < 0.05,
            "measured {measured} vs expected {expected}"
        );
    }

    #[test]
    fn preamp_scales_before_the_bands() {
        // Flat bands + preamp −6 dB ≈ ×0.5 on the settled sine.
        let mut eq = Eq3Band::new(RATE, 0.0, 0.0, 0.0, Volume::from_db(-6.0).unwrap()).unwrap();
        let mut planes = vec![sine(24_000, 997.0, 0.8)];
        eq.process(&mut planes).unwrap();
        let measured = settled_peak(&planes[0]);
        assert!(
            (measured - 0.4).abs() / 0.4 < 0.05,
            "measured {measured} vs expected 0.4"
        );
    }

    #[test]
    fn per_channel_state_is_independent() {
        // Same filter, different signals per channel: no cross-talk.
        let mut eq = Eq3Band::new(RATE, 6.0, 0.0, 0.0, Volume::UNITY).unwrap();
        let left = sine(24_000, 40.0, 0.5);
        let right = vec![0.0_f32; 24_000];
        let mut planes = vec![left, right];
        eq.process(&mut planes).unwrap();
        assert!(planes[1].iter().all(|s| *s == 0.0), "silence stays silent");
        assert!(settled_peak(&planes[0]) > 0.9, "left still boosts");
    }

    #[test]
    fn bad_parameters_are_typed_errors() {
        assert!(BiquadCoeffs::low_shelf(RATE, 0.0, 6.0, 0.707).is_err());
        assert!(BiquadCoeffs::low_shelf(RATE, 24_000.5, 6.0, 0.707).is_err());
        assert!(BiquadCoeffs::peaking(RATE, 1000.0, f32::NAN, 1.0).is_err());
        assert!(BiquadCoeffs::high_shelf(RATE, 4000.0, 6.0, 0.0).is_err());
    }

    #[test]
    fn eq_plugs_into_the_effect_chain() {
        use crate::EffectChain;
        let mut chain = EffectChain::new();
        chain
            .push(Box::new(Eq3Band::flat(RATE).unwrap()))
            .unwrap();
        let original = sine(600, 440.0, 0.3);
        let mut planes = vec![original.clone()];
        chain.process(&mut planes).unwrap();
        assert_eq!(planes[0], original);
    }
}
