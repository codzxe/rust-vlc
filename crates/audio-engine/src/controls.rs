//! The runtime master chain (FX3-A03 — wave-07's "dead audio-controls
//! surface" fix, engine half): session-level volume, mute, balance and
//! the three-band EQ, applied to the **mixed** quantum between the
//! mixer (stage C) and the device adaptation (stage D) — the effect
//! machinery's first streaming instantiation.
//!
//! # What was dead (W07-A02/A06/A07/A08)
//!
//! The audit's proof: `audio.set-volume` refused by every shipped host;
//! zero production callers of `set_volume`/`Gain`/`Eq3Band`; the mixer
//! permanently at [`Volume::UNITY`]; the EQ = real RBJ biquads wired to
//! nothing. All the machinery existed — [`Gain`], [`Mute`],
//! [`Balance`], [`Eq3Band`] — and no streaming path ever ran it. This
//! module is the wiring: [`AudioControls`] owns the concrete stages
//! and [`AudioPipeline`](crate::AudioPipeline) runs them per quantum.
//!
//! # Control discipline (the crate's own idiom)
//!
//! audio-engine is std-only with no threads and no channels (lib.rs
//! "# Never") — there is no message plumbing to extend. The crate's
//! control pattern is **direct setters with typed validation, called
//! by the host between quanta** (the `set_volume` / `set_drift` /
//! `set_channel_map` discipline), and this module mirrors it exactly:
//! every setter is a plain field update or an in-place coefficient
//! recompute — never a pipeline rebuild — validated at the control
//! boundary and observable through getters. A host seam (the CLI's
//! `AudioLimb`, a UI session) drives the same live pipeline object.
//!
//! # Zero behavior change when uncontrolled
//!
//! A fresh [`AudioControls`] is the exact identity: [`Volume::UNITY`]
//! gain, not muted, no balance stage, no EQ stage — and
//! [`AudioControls::process`] early-outs without touching one sample.
//! The uncontrolled pipeline is bit-identical to the pre-FX3-A03 path
//! (no loop runs, no allocation — the quantum zero-allocation proof
//! keeps holding untouched).
//!
//! # Why concrete stages, not `Box<dyn AudioEffect>`
//!
//! [`EffectChain`](crate::EffectChain) stays the generic bounded
//! composition for fully-static chains; a runtime-controllable chain
//! needs to *reach* its stages' parameters (a volume change, a band
//! re-arm), and a boxed trait object hides them. [`AudioControls`]
//! holds the concrete types and reuses their `AudioEffect::process`
//! implementations verbatim — no second DSP, the audit's
//! "two implementations of apply-a-Volume" criticism does not recur.
//!
//! # Stage order (fixed by design, like the chain's push order)
//!
//! gain → mute → balance → EQ: the volume scalar, then the mute zero,
//! then the constant-power balance split, then the preamp + three
//! biquads. A muted chain zeroes everything downstream of it; the EQ
//! runs last, on the balanced signal at the set loudness (its preamp
//! is the trim-into-the-EQ knob). Balance center is the pan law's
//! −3.01 dB per channel, **not** unity — the stage exists only once a
//! position is set ([`clear_balance`](AudioControls::clear_balance)
//! removes it; a host that wants a plain pass-through never arms it).
//!
//! # Hot path
//!
//! No allocation in `process` (the EQ's per-channel state warms on the
//! first quantum — the pipeline's stage-A capacity discipline; the
//! identity early-out skips even the loops). Setters may recompute
//! biquad coefficients in place; the EQ's filter state is preserved
//! across a parameter change (a coefficient swap, glitch-bounded, not
//! a rebuild — see [`Eq3Band::set_bands`]).

use media_types::EngineError;

use crate::effects::{AudioEffect, Balance, Eq3Band, Gain, Mute};
use crate::volume::Volume;

/// The session master chain — the runtime audio controls (see the
/// module docs for the discipline and the stage order). Owned by
/// [`AudioPipeline`](crate::AudioPipeline); every setter is safe to
/// call between quanta on the live pipeline.
#[derive(Debug, Clone)]
pub struct AudioControls {
    /// Master volume (the [`Gain`] stage's scalar; `UNITY` = identity).
    gain: Gain,
    /// The mute flag (`false` = the stage is absent).
    muted: bool,
    /// The constant-power balance stage (`None` = absent — center is
    /// −3.01 dB per channel, so it arms only when a host asks).
    balance: Option<Balance>,
    /// The three-band EQ (`None` = absent).
    eq: Option<Eq3Band>,
    /// The session rate — the rate `Eq3Band` arms and recomputes at.
    rate: u32,
}

impl AudioControls {
    /// The identity controls for a session at `rate`: unity gain, not
    /// muted, no balance, no EQ (see [`is_identity`](Self::is_identity)
    /// — nothing runs until a control arrives).
    #[must_use]
    pub fn new(rate: u32) -> AudioControls {
        AudioControls {
            gain: Gain::new(Volume::UNITY),
            muted: false,
            balance: None,
            eq: None,
            rate,
        }
    }

    /// The session rate the EQ arms at.
    #[must_use]
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Master volume — a plain stage swap ([`Gain`] is a copyable
    /// scalar holder; no allocation, no rebuild). `Volume` carries its
    /// own construction validation; [`Volume::UNITY`] restores the
    /// identity.
    pub fn set_volume(&mut self, volume: Volume) {
        self.gain = Gain::new(volume);
    }

    /// The master volume in force.
    #[must_use]
    pub fn volume(&self) -> Volume {
        self.gain.volume()
    }

    /// Mute. `true` zeroes every mixed quantum; `false` restores the
    /// signal bit-exactly (the stage is a stateless multiply by zero —
    /// no fade, no residue).
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Whether the chain is muted.
    #[must_use]
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// Arm the constant-power balance stage at `position` (−1.0 hard
    /// left … 0.0 center … +1.0 hard right; the pan law's exact math
    /// lives on [`Balance`]).
    ///
    /// # Errors
    /// `audio.effects_balance_position` for non-finite or
    /// out-of-range positions (the knob's hard stops — D-6, never
    /// silent clamping).
    pub fn set_balance(&mut self, position: f32) -> Result<(), EngineError> {
        self.balance = Some(Balance::new(position)?);
        Ok(())
    }

    /// Remove the balance stage (back to the identity split).
    pub fn clear_balance(&mut self) {
        self.balance = None;
    }

    /// The balance position in force, `None` when the stage is absent.
    #[must_use]
    pub fn balance(&self) -> Option<f32> {
        self.balance.as_ref().map(Balance::position)
    }

    /// Arm (first call) or update (later calls) the three-band EQ.
    /// Arming builds the filters at the session rate; updating
    /// recomputes the three coefficient sets **in place, preserving
    /// the biquad state** — a parameter change on the live chain,
    /// never a rebuild (see [`Eq3Band::set_bands`]).
    ///
    /// # Errors
    /// `audio.effects_eq_params` for bad band parameters (non-finite
    /// gains, |gain| beyond ±40 dB — the runtime bound; a session rate
    /// too low for the fixed 4 kHz shelf corner refuses typed).
    pub fn set_eq(
        &mut self,
        low_db: f32,
        mid_db: f32,
        high_db: f32,
        preamp: Volume,
    ) -> Result<(), EngineError> {
        match self.eq.as_mut() {
            Some(eq) => eq.set_bands(low_db, mid_db, high_db, preamp),
            None => {
                self.eq = Some(Eq3Band::new(self.rate, low_db, mid_db, high_db, preamp)?);
                Ok(())
            }
        }
    }

    /// Remove the EQ stage (back to the identity).
    pub fn clear_eq(&mut self) {
        self.eq = None;
    }

    /// The EQ configuration in force — (low, mid, high dB, preamp) —
    /// `None` when the stage is absent.
    #[must_use]
    pub fn eq_bands(&self) -> Option<(f32, f32, f32, Volume)> {
        self.eq
            .as_ref()
            .map(|eq| (eq.low_db(), eq.mid_db(), eq.high_db(), eq.preamp()))
    }

    /// Whether the chain is the exact identity (no stage would run —
    /// the uncontrolled default).
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.gain.volume() == Volume::UNITY
            && !self.muted
            && self.balance.is_none()
            && self.eq.is_none()
    }

    /// Run the armed stages over one quantum's mixed planes, in the
    /// documented order (gain → mute → balance → EQ). The identity
    /// configuration touches nothing — the early-out is what makes an
    /// uncontrolled pipeline bit-identical to the pre-FX3-A03 path.
    ///
    /// # Errors
    /// The balance stage's stereo gate
    /// (`audio.effects_balance_channels`) on a non-stereo plane set —
    /// [`AudioPipeline::set_balance`](crate::AudioPipeline::set_balance)
    /// pre-validates the negotiated shape, so a session that took the
    /// setter's `Ok` never fails here.
    pub fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError> {
        if self.is_identity() {
            return Ok(());
        }
        self.gain.process(planes)?;
        if self.muted {
            let mut mute = Mute;
            mute.process(planes)?;
        }
        if let Some(balance) = self.balance.as_mut() {
            balance.process(planes)?;
        }
        if let Some(eq) = self.eq.as_mut() {
            eq.process(planes)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// A one-channel sine of `n` samples at amplitude `a` (integer
    /// periods keep the peak measurable by max-abs — the effects
    /// module's own convention).
    fn sine(n: usize, a: f32) -> Vec<f32> {
        (0..n)
            .map(|i| a * (2.0 * std::f32::consts::PI * i as f32 / 48.0).sin())
            .collect()
    }

    fn peak(plane: &[f32]) -> f32 {
        plane.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    #[test]
    fn fx3_a03_controls_identity_is_bit_exact_noop() {
        // The acceptance bar: a fresh chain touches nothing.
        let mut controls = AudioControls::new(48_000);
        assert!(controls.is_identity());
        let original = vec![sine(48, 0.75), sine(48, -0.5)];
        let mut planes = original.clone();
        controls.process(&mut planes).must();
        assert_eq!(planes, original, "identity must be bit-preserving");
    }

    #[test]
    fn fx3_a03_controls_volume_mute_balance_compose_in_order() {
        // Volume 0.5 then hard-left balance: L halves, R zeroes; mute
        // then kills everything; unmuting restores exactly (stateless).
        let mut controls = AudioControls::new(48_000);
        controls.set_volume(Volume::from_linear(0.5).must());
        controls.set_balance(-1.0).must();
        let mut planes = vec![sine(48, 0.8), sine(48, 0.8)];
        controls.process(&mut planes).must();
        assert!((peak(&planes[0]) - 0.4).abs() < 1e-6, "L: 0.8×0.5");
        assert!(peak(&planes[1]) < 1e-6, "hard-left kills R");
        controls.set_muted(true);
        let mut planes = vec![sine(48, 0.8), sine(48, 0.8)];
        controls.process(&mut planes).must();
        assert!(planes.iter().all(|p| p.iter().all(|s| *s == 0.0)));
        // Unmute restores the signal exactly (stateless ×0) — a fresh
        // copy of the same input re-processes to the same output.
        controls.set_muted(false);
        let mut planes = vec![sine(48, 0.8), sine(48, 0.8)];
        controls.process(&mut planes).must();
        assert!((peak(&planes[0]) - 0.4).abs() < 1e-6, "unmute restores");
    }

    #[test]
    fn fx3_a03_controls_flat_eq_is_bit_exact_and_getters_track() {
        // A flat EQ is the exact identity filter (eq.rs's pinned
        // property), and the configuration getters mirror every set.
        let mut controls = AudioControls::new(48_000);
        controls.set_eq(0.0, 0.0, 0.0, Volume::UNITY).must();
        assert!(!controls.is_identity(), "the stage is armed");
        assert_eq!(controls.eq_bands(), Some((0.0, 0.0, 0.0, Volume::UNITY)));
        let original = sine(48, 0.6);
        let mut planes = vec![original.clone()];
        controls.process(&mut planes).must();
        assert_eq!(planes[0], original, "flat EQ must be bit-exact");
        // The runtime update seam: parameters change on the armed chain.
        controls.set_eq(0.0, -6.0, 0.0, Volume::UNITY).must();
        assert_eq!(controls.eq_bands().map(|b| b.1), Some(-6.0));
        controls.clear_eq();
        assert_eq!(controls.eq_bands(), None);
        controls.clear_balance();
        controls.set_volume(Volume::UNITY);
        controls.set_muted(false);
        assert!(controls.is_identity());
    }

    #[test]
    fn fx3_a03_controls_validation_is_typed() {
        let mut controls = AudioControls::new(48_000);
        // Balance hard stops.
        let err = controls.set_balance(1.01).err().must();
        assert_eq!(err.context.code, "audio.effects_balance_position");
        let err = controls.set_balance(f32::NAN).err().must();
        assert_eq!(err.context.code, "audio.effects_balance_position");
        assert_eq!(controls.balance(), None, "failed arm leaves no stage");
        // The EQ runtime bound.
        let err = controls.set_eq(41.0, 0.0, 0.0, Volume::UNITY).err().must();
        assert_eq!(err.context.code, "audio.effects_eq_params");
        let err = controls
            .set_eq(0.0, f32::NAN, 0.0, Volume::UNITY)
            .err()
            .must();
        assert_eq!(err.context.code, "audio.effects_eq_params");
        assert_eq!(controls.eq_bands(), None, "failed arm leaves no stage");
        // The balance stereo gate at process time (the pipeline setter
        // pre-validates; standalone use surfaces the stage's own gate).
        let mut stereo_gate = AudioControls::new(48_000);
        stereo_gate.set_balance(0.0).must();
        let err = stereo_gate.process(&mut [sine(8, 0.1)]).err().must();
        assert_eq!(err.context.code, "audio.effects_balance_channels");
    }
}
