//! The audio effect chain (issue #33 / D-012 — the filter-chain card's
//! v0 core: gain, balance/pan, mute, with the chain abstraction the
//! EQ/compressor cards plug into).
//!
//! # Shape
//!
//! [`AudioEffect`] is a per-track, ordered, **bounded** stage over f32
//! planar samples — the mixer format stage B hands over (doc 03 §4.8
//! b's insertion point B, empty in card 2.C3, starts here). Every
//! effect mutates its channel planes **in place**: the process loop
//! allocates nothing (D-1), so the hot path is a plain multiply or a
//! two-gain split per sample.
//!
//! [`EffectChain`] is the ordered composition: at most
//! [`MAX_EFFECTS`] stages (bounded memory by construction), pushed in
//! execution order, processed in that order. An empty chain is the
//! identity (a no-op loop, not a copy).
//!
//! # Pan law (documented, tested)
//!
//! [`Balance`] uses the **constant-power** law: position `p ∈
//! [−1.0, +1.0]` maps to channel gains
//!
//! ```text
//! L = cos((p + 1) · π/4)    R = sin((p + 1) · π/4)
//! ```
//!
//! so `p = 0` gives L = R = √2/2 (−3.01 dB each, total power
//! preserved), `p = −1` gives (1, 0) and `p = +1` gives (0, 1). The
//! constants live as associated consts ([`Balance::UNITY_GAIN`],
//! [`Balance::LEFT_GAIN`], [`Balance::RIGHT_GAIN`]) so the pan-law
//! contract is pinned by tests, not prose. Stereo only: a Balance
//! stage on a non-stereo plane set is a typed
//! `audio.effects_balance_channels` error, never a silent mono fold.
//!
//! # Never
//!
//! Allocation in `process`, panics, unwraps, NaN gains (rejected at
//! construction, the [`Volume`] discipline).

use media_types::EngineError;

use crate::error;
use crate::volume::Volume;

pub mod eq;

pub use eq::{BiquadCoeffs, BiquadFilter, Eq3Band};

/// The chain's hard bound (D-2 bounded memory): more stages than this
/// is a configuration bug, not a runtime growth path.
pub const MAX_EFFECTS: usize = 8;

/// One ordered DSP stage over f32 planar samples.
///
/// `planes[k]` is channel k's sample plane; all planes in one call
/// have the same length (the pipeline's invariant — effects rely on
/// it and validate channel *counts*, not per-plane length splits,
/// which the pipeline already guarantees at stage A).
pub trait AudioEffect {
    /// Process one quantum's planes, in place. Idempotent per quantum;
    /// stateless stages must be exactly re-runnable.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with a
    /// stable `audio.effects_*` context code for shape violations
    /// (e.g. stereo-only effects on a different channel count).
    fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError>;

    /// A human-readable stage name (diagnostics; no allocation in the
    /// hot path — it is a `&'static str`).
    fn name(&self) -> &'static str;
}

/// An ordered, bounded chain of [`AudioEffect`] stages (the
/// filter-chain seam's v0: gain → balance → … in push order).
///
/// The chain holds boxed trait objects but is itself bounded by
/// [`MAX_EFFECTS`] at push time — the only allocation is the box at
/// *configuration* time, never inside `process`.
pub struct EffectChain {
    effects: Vec<Box<dyn AudioEffect>>,
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectChain {
    /// An empty chain (the identity).
    pub fn new() -> Self {
        EffectChain {
            effects: Vec::with_capacity(MAX_EFFECTS),
        }
    }

    /// Append a stage; runs after every stage already pushed.
    ///
    /// # Errors
    /// `audio.effects_chain_full` once [`MAX_EFFECTS`] stages are
    /// installed — the bound is a wall, not a hint.
    pub fn push(&mut self, effect: Box<dyn AudioEffect>) -> Result<(), EngineError> {
        if self.effects.len() >= MAX_EFFECTS {
            return Err(error::unsupported_format("audio.effects_chain_full"));
        }
        self.effects.push(effect);
        Ok(())
    }

    /// How many stages are installed.
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Whether the chain is the identity (no stages).
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Drop every stage (back to the identity).
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Run every stage, in order. The empty chain is a no-op.
    ///
    /// # Errors
    /// The first failing stage's error, unchanged (fail-fast: later
    /// stages never see a half-processed quantum).
    pub fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError> {
        for effect in &mut self.effects {
            effect.process(planes)?;
        }
        Ok(())
    }
}

/// A gain stage: one [`Volume`] scalar multiply on every sample of
/// every channel (the effect-chain's front door for `audio.set-volume`
/// — engine-side it is the same multiply the mixer applies, exposed as
/// an ordered stage so filter cards compose with it deterministically).
#[derive(Debug, Clone, Copy)]
pub struct Gain {
    volume: Volume,
}

impl Gain {
    /// A gain stage at `volume` (finite, non-negative by [`Volume`]'s
    /// own construction discipline).
    pub fn new(volume: Volume) -> Self {
        Gain { volume }
    }

    /// The stage's volume.
    pub fn volume(&self) -> Volume {
        self.volume
    }
}

impl AudioEffect for Gain {
    fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError> {
        let g = self.volume.linear();
        for plane in planes {
            for sample in plane.iter_mut() {
                *sample *= g;
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "gain"
    }
}

/// A mute stage: multiply by exactly zero (the [`Volume::MUTE`]
/// semantics — a NaN sample's payload sign is preserved, every finite
/// sample becomes +0.0; un-muting is removing the stage, not a state
/// flip inside it, so the chain stays a pure function of its config).
#[derive(Debug, Clone, Copy, Default)]
pub struct Mute;

impl AudioEffect for Mute {
    fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError> {
        for plane in planes {
            for sample in plane.iter_mut() {
                *sample *= 0.0;
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "mute"
    }
}

/// Constant-power stereo balance (see the module's pan-law section).
#[derive(Debug, Clone, Copy)]
pub struct Balance {
    position: f32,
}

impl Balance {
    /// Center: both channels √2/2 ≈ 0.70710678 (−3.01 dB).
    pub const UNITY_GAIN: f32 = std::f32::consts::FRAC_1_SQRT_2;

    /// Hard left: (1.0, 0.0) — the law's exact math; f32 `cos(π/2)`
    /// lands within 1e-7 of zero (trig rounding, pinned by test).
    pub const LEFT_GAIN: (f32, f32) = (1.0, 0.0);

    /// Hard right: (0.0, 1.0) — same f32 note as [`LEFT_GAIN`].
    pub const RIGHT_GAIN: (f32, f32) = (0.0, 1.0);

    /// A balance position, `−1.0` (full left) through `0.0` (center)
    /// to `+1.0` (full right).
    ///
    /// # Errors
    /// `audio.effects_balance_position` for non-finite or out-of-range
    /// positions (the pan knob has hard stops; no silent clamping of a
    /// control value — D-6's "never invent" applies to DSP too).
    pub fn new(position: f32) -> Result<Balance, EngineError> {
        if !position.is_finite() || !(-1.0..=1.0).contains(&position) {
            return Err(error::bad_gain("audio.effects_balance_position"));
        }
        Ok(Balance { position })
    }

    /// The position this stage was built with.
    pub fn position(&self) -> f32 {
        self.position
    }

    /// The (left, right) channel gains the constant-power law gives
    /// this position: `cos/sin` of `(p+1)·π/4`.
    pub fn gains(&self) -> (f32, f32) {
        let theta = (self.position + 1.0) * std::f32::consts::FRAC_PI_4;
        (theta.cos(), theta.sin())
    }
}

impl AudioEffect for Balance {
    fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError> {
        if planes.len() != 2 {
            return Err(error::channel_range("audio.effects_balance_channels"));
        }
        let (lg, rg) = self.gains();
        // Two disjoint mutable planes via split_at_mut (no overlap,
        // no re-borrow of the whole slice).
        let (lefts, rights) = planes.split_at_mut(1);
        let left = &mut lefts[0];
        let right = &mut rights[0];
        // Equal plane lengths are the pipeline's stage-A invariant;
        // zip (not index) keeps this safe even if a future caller
        // breaks it — the shorter length wins, never an OOB panic.
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            *l *= lg;
            *r *= rg;
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "balance"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// A one-channel sine of `n` samples at amplitude `a` (integer
    /// periods keep the peak measurable by max-abs, not by phase luck).
    fn sine(n: usize, a: f32) -> Vec<f32> {
        (0..n)
            .map(|i| a * (2.0 * std::f32::consts::PI * i as f32 / 48.0).sin())
            .collect()
    }

    fn peak(plane: &[f32]) -> f32 {
        plane.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    #[test]
    fn gain_scales_sine_amplitude() {
        // Known signal in → known signal out: sine amplitude 0.5
        // through gain −6.02 dB (linear ≈ 0.5) halves the amplitude.
        let mut gain = Gain::new(Volume::from_db(-6.0206).must());
        let mut planes = vec![sine(48, 0.5)];
        gain.process(&mut planes).must();
        let expected = 0.5 * Volume::from_db(-6.0206).must().linear();
        assert!(
            (peak(&planes[0]) - expected).abs() < 1e-3,
            "peak {} vs expected {}",
            peak(&planes[0]),
            expected
        );
    }

    #[test]
    fn gain_identity_is_bit_exact() {
        let mut gain = Gain::new(Volume::UNITY);
        let original = sine(48, 0.75);
        let mut planes = vec![original.clone()];
        gain.process(&mut planes).must();
        assert_eq!(planes[0], original, "unity gain must be bit-preserving");
    }

    #[test]
    fn mute_zeroes_everything() {
        let mut planes = vec![sine(48, 0.9), sine(48, 0.9)];
        Mute.process(&mut planes).must();
        assert!(planes.iter().all(|p| p.iter().all(|s| *s == 0.0)));
    }

    #[test]
    fn balance_center_is_minus_3db_constant_power() {
        let b = Balance::new(0.0).must();
        let (l, r) = b.gains();
        assert!((l - Balance::UNITY_GAIN).abs() < 1e-6);
        assert!((r - Balance::UNITY_GAIN).abs() < 1e-6);
        // Constant power: L² + R² = 1 at every position on the law.
        let (l2, r2) = Balance::new(0.37).must().gains();
        assert!((l2 * l2 + r2 * r2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn balance_hard_stops_are_pinned() {
        // f32 trig: cos(π/2) = −4.37e-8, not 0 — the law's extremes
        // are pinned to 1e-7, the honest tolerance.
        let (l, r) = Balance::new(-1.0).must().gains();
        assert!((l - Balance::LEFT_GAIN.0).abs() < 1e-7);
        assert!((r - Balance::LEFT_GAIN.1).abs() < 1e-7);
        let (l, r) = Balance::new(1.0).must().gains();
        assert!((l - Balance::RIGHT_GAIN.0).abs() < 1e-7);
        assert!((r - Balance::RIGHT_GAIN.1).abs() < 1e-7);
    }

    #[test]
    fn balance_scales_each_channel_by_its_law_gain() {
        // Sine L=0.8, R=0.2 through hard-left: L unchanged, R zero.
        let mut planes = vec![sine(48, 0.8), sine(48, 0.2)];
        Balance::new(-1.0).must().process(&mut planes).must();
        assert!((peak(&planes[0]) - 0.8).abs() < 1e-6);
        assert!(peak(&planes[1]) < 1e-7);
    }

    #[test]
    fn balance_rejects_out_of_range_and_non_stereo() {
        assert!(Balance::new(1.01).is_err());
        assert!(Balance::new(f32::NAN).is_err());
        assert!(
            Balance::new(0.0)
                .must()
                .process(&mut [sine(8, 0.1)])
                .is_err()
        );
        let mut mono = vec![sine(8, 0.1)];
        assert!(Balance::new(0.0).must().process(&mut mono).is_err());
    }

    #[test]
    fn chain_runs_stages_in_push_order() {
        // gain 0.5 then balance hard-right: L halves then zeroes,
        // R halves then keeps full scale — order matters and is
        // observable (gain→balance ≠ balance→gain at the extremes is
        // *not* distinguishable, so this pins the composition value
        // itself: 0.4·0.5 on R with L fully attenuated).
        let mut chain = EffectChain::new();
        chain
            .push(Box::new(Gain::new(Volume::from_linear(0.5).must())))
            .must();
        chain.push(Box::new(Balance::new(1.0).must())).must();
        let mut planes = vec![sine(48, 0.4), sine(48, 0.4)];
        chain.process(&mut planes).must();
        assert!(peak(&planes[0]) < 1e-7, "hard-right kills L");
        assert!((peak(&planes[1]) - 0.2).abs() < 1e-6);
    }

    #[test]
    fn empty_chain_is_identity_noop() {
        let mut chain = EffectChain::new();
        let original = sine(16, 0.3);
        let mut planes = vec![original.clone()];
        chain.process(&mut planes).must();
        assert_eq!(planes[0], original);
    }

    #[test]
    fn chain_bound_is_a_wall() {
        let mut chain = EffectChain::new();
        for _ in 0..MAX_EFFECTS {
            chain.push(Box::new(Mute)).must();
        }
        assert_eq!(chain.len(), MAX_EFFECTS);
        assert!(chain.push(Box::new(Mute)).is_err());
        chain.clear();
        assert!(chain.is_empty());
    }

    #[test]
    fn gain_mute_compose_to_silence() {
        let mut chain = EffectChain::new();
        chain
            .push(Box::new(Gain::new(Volume::from_linear(2.0).must())))
            .must();
        chain.push(Box::new(Mute)).must();
        let mut planes = vec![sine(48, 0.5)];
        chain.process(&mut planes).must();
        assert!(planes[0].iter().all(|s| *s == 0.0));
    }
}
