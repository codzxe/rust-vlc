//! The audio-side drift governor (card 3.B1; W1-18 §3.2) —
//! **consumer #2 of [`DriftCorrection`]**, the derivation site audit
//! M12 named absent: the component that *measures* the device-vs-
//! nominal divergence and *derives* a bounded correction from it via
//! [`DriftCorrection::for_divergence`].
//!
//! # The plant (why fill is the error signal)
//!
//! The device consumes `C(t)` sample instants (the counter — the
//! clock's truth, doc 03 §4.9); the pump stages quanta into the SPSC
//! ring. Ring fill therefore integrates `(production − consumption)`:
//! a device-vs-nominal rate error of `δ` ppm drains (or floods) the
//! ring at `δ` µs/s until underrun or `audio.ring_full`. Because the
//! plant *integrates* rate error, a proportional-on-position
//! controller is integral-in-the-loop: the correction that holds the
//! fill steady is exactly the device's rate error, and the fill error
//! settles at the small proportional offset `−W_c · δ` (2 ms at
//! 100 ppm, 20 ms at the ±1000 ppm bound — inside the design's
//! acceptance band `[10 ms, 150 ms]` of fill around the 40 ms
//! target). The fill is also an A/V error term: audio at the speaker
//! lags the clock by the fill, so regulating fill bounds A/V error.
//!
//! # The controller
//!
//! Per 1 s window (a [`RateWindow`] over the injected time source),
//! compute `c = DriftCorrection::for_divergence(fill − target, W_c)`
//! with integral time `W_c = 20 s` `[PROPOSAL per W1-18 §3.2 — the
//! limiter is the invariant (drift.rs:23-25), the time constant is a
//! Phase-3-measurable number]`, then apply the step limiter (≤ 250
//! ppm change per update `[PROPOSAL]`): the controller is a pure
//! function of `e` plus a step limit — **no wind-up state exists**
//! (saturation holds at the bound by construction of
//! [`DriftCorrection::new`]).
//!
//! # Time source discipline (W1-18 §2.2)
//!
//! `now_ns` is **injected** (the events.rs:41-47 doctrine). The
//! wiring derives it from the *device* time base (samples advanced ×
//! 1e9 / rate): it advances exactly when the device pulls, and a
//! frozen device (paused / underrun-held) yields a frozen window —
//! zero updates, deterministically (the `ProgressWindow`
//! frozen-source property, mirrored and tested in media-clock's
//! `sync` module). Media-position time likewise freezes on pause;
//! ambient wall time must NOT drive the window — it would keep
//! evaluating while paused.
//!
//! Unmeasurable latency (`fill = None`) parks the governor at
//! [`DriftCorrection::ZERO`] — honest degradation, never a fabricated
//! estimate (MP §8).
//!
//! # Observability (no event storms, W1-18 §5)
//!
//! Everything here is counters, polled through [`DriftStats`] — the
//! doc 05 catalog is frozen (D-017) and drift is not an error:
//! nothing touches `playback.error` or any coalescer.

use media_clock::sync::{DriftEstimator, RateWindow};
use media_clock::{DriftCorrection, MAX_CORRECTION_PPM};

/// The correction re-evaluation window `[PROPOSAL per W1-18 §3.2]`:
/// 1 s of injected device time — the ≤1 Hz bound.
pub const CORRECTION_WINDOW_NS: u64 = 1_000_000_000;

/// The integral time `W_c` `[PROPOSAL per W1-18 §3.2]`: 20 s — the
/// divergence-to-correction divisor (the fill error's proportional
/// band; the steady-state offset is `W_c · |δ|`, 2 ms at 100 ppm).
pub const INTEGRAL_TIME_NS: u64 = 20_000_000_000;

/// The step limiter `[PROPOSAL per W1-18 §3.2]`: at most 250 ppm of
/// correction change per re-evaluation (slack resets each window).
pub const STEP_LIMIT_PPM: i64 = 250;

/// The governor's runtime state (fixed-size, zero-allocation — D-1).
#[derive(Debug, Clone)]
pub struct DriftGovernor {
    /// The wiring's fill target (ns) — the regulated operating point.
    target_ns: u64,
    /// The correction currently in force (step-limited).
    correction: DriftCorrection,
    /// The ≤1 Hz re-evaluation window over the injected time source.
    window: RateWindow,
    /// Whether the window has been opened by a first sample (the
    /// opening call carries one sample — not enough to correct on;
    /// the first evaluation waits for a full window of them).
    opened: bool,
    /// The fill-error estimator (EMA + windowed slope, for stats).
    estimator: DriftEstimator,
    /// Polled observability (never pushed — no event storms).
    stats: DriftStats,
}

/// The governor's polled observability (W1-18 §5: counters only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DriftStats {
    /// Fill samples taken (measurable ones).
    pub samples: u64,
    /// Samples the sink could not answer (`latency() == None`) — the
    /// honest-degradation count.
    pub unmeasurable: u64,
    /// Correction re-evaluations performed.
    pub updates: u64,
    /// Times the desired correction hit the ±1000 ppm limiter (the
    /// device is beyond the correctable range — the structural
    /// ceiling's early warning).
    pub clamped: u64,
    /// Times the step limiter held back the desired change.
    pub step_limited: u64,
    /// The last measured fill (ns).
    pub fill_ns_last: Option<u64>,
    /// The smallest fill ever seen (ns; underrun proximity).
    pub fill_ns_min: Option<u64>,
    /// The largest fill ever seen (ns; flood proximity).
    pub fill_ns_max: Option<u64>,
    /// The last error sample `fill − target` (ns).
    pub error_ns_last: Option<i64>,
    /// Estimator/governor resets (generation / anchor-epoch bumps).
    pub resets: u64,
    /// The last observed rate mismatch (the estimator's slope, ppm) —
    /// an observability number, never a control input (W1-18 §2.2).
    pub slope_ppm_last: Option<i64>,
}

impl DriftGovernor {
    /// A governor regulating toward `target_ns` of ring fill, with the
    /// W1-18 `[PROPOSAL]` constants (1 s window, 20 s integral time,
    /// 250 ppm step limit — the *invariants* are the ±1000 ppm
    /// limiter and bounded corrections, not these numbers).
    #[must_use]
    pub fn new(target_ns: u64) -> DriftGovernor {
        DriftGovernor {
            target_ns,
            correction: DriftCorrection::ZERO,
            window: RateWindow::new(CORRECTION_WINDOW_NS),
            opened: false,
            estimator: DriftEstimator::new(),
            stats: DriftStats::default(),
        }
    }

    /// The correction in force (the actuator output — feed it to the
    /// micro-resampler).
    #[must_use]
    pub fn correction(&self) -> DriftCorrection {
        self.correction
    }

    /// The fill target in force (ns).
    #[must_use]
    pub fn target_ns(&self) -> u64 {
        self.target_ns
    }

    /// The polled observability (counters only — never an event).
    #[must_use]
    pub fn stats(&self) -> DriftStats {
        self.stats
    }

    /// Observe one fill sample on the per-quantum cadence and answer
    /// the correction in force afterwards (the wiring applies it via
    /// `set_correction` on the pipeline's resampler).
    ///
    /// * `now_ns` — the injected device-time stamp (freezes ⇒ no
    ///   evaluation, deterministically; backwards jumps merely hold).
    /// * `fill_ns` — `AudioSink::latency()`'s answer, or `None` when
    ///   the sink cannot measure (park at zero — honest degradation).
    ///
    /// Re-evaluation happens only when the 1 s window permits **and
    /// a first window has fully elapsed** (the opening sample alone
    /// carries no trend); the desired correction is derived from the
    /// estimator's **filtered** error — the EMA over per-quantum
    /// samples (τ ≈ 267 ms at the 30 Hz quantum cadence) — so
    /// per-quantum fill jitter reaches the control law attenuated,
    /// never amplified (S5's criterion). Every call feeds the
    /// estimator and the stats.
    pub fn observe(&mut self, now_ns: u64, fill_ns: Option<u64>) -> DriftCorrection {
        let Some(fill) = fill_ns else {
            self.stats.unmeasurable = self.stats.unmeasurable.saturating_add(1);
            self.correction = DriftCorrection::ZERO;
            return self.correction;
        };
        self.stats.samples = self.stats.samples.saturating_add(1);
        self.stats.fill_ns_last = Some(fill);
        self.stats.fill_ns_min = Some(self.stats.fill_ns_min.map_or(fill, |m| m.min(fill)));
        self.stats.fill_ns_max = Some(self.stats.fill_ns_max.map_or(fill, |m| m.max(fill)));
        let error = (i128::from(fill) - i128::from(self.target_ns))
            .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        self.stats.error_ns_last = Some(error);
        self.estimator.observe_windowed(now_ns, error);
        self.stats.slope_ppm_last = self.estimator.slope_ppm();

        if !self.opened {
            // The window opens on the first sample; the opening call
            // itself does not evaluate (one sample is no trend).
            self.opened = true;
            let _ = self.window.permits(now_ns);
        } else if self.window.permits(now_ns) {
            // Pure function of the filtered error plus the step
            // limiter (no wind-up state): desired =
            // for_divergence(ê, W_c), then bound the change to
            // ±STEP_LIMIT_PPM around the correction in force.
            let filtered = self.estimator.error_ema().unwrap_or(error);
            let desired = DriftCorrection::for_divergence(i128::from(filtered), INTEGRAL_TIME_NS);
            if desired.ppm().abs() >= MAX_CORRECTION_PPM {
                self.stats.clamped = self.stats.clamped.saturating_add(1);
            }
            let old = self.correction.ppm();
            let lo = old.saturating_sub(STEP_LIMIT_PPM);
            let hi = old.saturating_add(STEP_LIMIT_PPM);
            let stepped = desired.ppm().clamp(lo, hi);
            if stepped != desired.ppm() {
                self.stats.step_limited = self.stats.step_limited.saturating_add(1);
            }
            self.correction = DriftCorrection::new(stepped);
            self.stats.updates = self.stats.updates.saturating_add(1);
        }
        self.correction
    }

    /// A new regime (generation change / anchor-epoch bump — W1-18
    /// §2.2): the fill-to-target relationship restarts, so the
    /// measurement restarts. The applied correction drops to zero
    /// with the measurement (the post-regime transient is bounded by
    /// the step limiter: at most one `STEP_LIMIT_PPM` move on the
    /// first re-evaluation — the S7 criterion).
    pub fn reset(&mut self) {
        self.correction = DriftCorrection::ZERO;
        self.window.reset();
        self.opened = false;
        self.estimator.reset();
        self.stats.resets = self.stats.resets.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    const TARGET_MS: i64 = 40;
    const TARGET_NS: i64 = TARGET_MS * 1_000_000;

    fn governor() -> DriftGovernor {
        DriftGovernor::new(TARGET_NS as u64)
    }

    /// The S1–S3 plant model, shared by the loop tests below: the
    /// device consumes `drift_ppm` fast (positive = draining), the
    /// production side applies the governor's correction. One virtual
    /// second per step, quantum = 33.33 ms (30 observes per second —
    /// the per-quantum cadence at 44.1 kHz / 1470).
    struct Plant {
        drift_ppm: i64,
        /// Fill in ns (starts at target: a primed ring).
        fill_ns: i64,
        /// Virtual device time (ns).
        now_ns: u64,
        quantum_ns: u64,
    }

    impl Plant {
        fn new(drift_ppm: i64) -> Plant {
            Plant {
                drift_ppm,
                fill_ns: TARGET_NS,
                now_ns: 0,
                quantum_ns: 33_333_333,
            }
        }

        /// One quantum: the governor observes, the plant integrates
        /// `(correction − drift)` over the quantum, time advances.
        fn step(&mut self, governor: &mut DriftGovernor) -> DriftCorrection {
            let correction = governor.observe(self.now_ns, Some(self.fill_ns.unsigned_abs()));
            let rate_delta_ppm = correction.ppm() - self.drift_ppm;
            // ns of fill change per ns of time = rate_delta × 1e-3.
            let delta = (i128::from(rate_delta_ppm) * i128::from(self.quantum_ns)) / 1_000_000;
            self.fill_ns = self
                .fill_ns
                .saturating_add(delta.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64);
            self.now_ns += self.quantum_ns;
            correction
        }
    }

    #[test]
    fn unmeasurable_sink_parks_at_zero() {
        let mut g = governor();
        // Prime a nonzero correction first with measurable fills.
        let mut plant = Plant::new(600);
        for _ in 0..90 {
            let _ = plant.step(&mut g);
        }
        assert_ne!(g.correction().ppm(), 0);
        // The sink goes silent: park at zero, count it.
        let c = g.observe(10_000_000_000, None);
        assert_eq!(c, DriftCorrection::ZERO);
        assert_eq!(g.stats().unmeasurable, 1);
    }

    #[test]
    fn s1_fixed_plus_100ppm_device_converges_within_45s() {
        // Scenario S1 (W1-18 §6): +100 ppm draining device, 120 s
        // virtual, quantum 33.33 ms. The ring starts primed at the
        // target; at t = 60 s a −25 ms fill disturbance (a startup-
        // shaped drop) tests the *return*: |e| ≤ 10 ms within 45 s of
        // the disturbance, then stays to the end. Zero underruns
        // (fill > 0 always), zero floods (fill inside the acceptance
        // band's ceiling), correction always within ±1000 ppm.
        let mut g = governor();
        let mut plant = Plant::new(100);
        let mut disturbed = false;
        let mut returned_at: Option<u64> = None;
        let mut clamped_before_disturbance = 0_u64;
        for quantum in 0..(120 * 30) {
            if quantum == 60 * 30 {
                clamped_before_disturbance = g.stats().clamped;
                plant.fill_ns -= 25_000_000;
                disturbed = true;
            }
            let c = plant.step(&mut g);
            assert!(
                c.ppm().abs() <= MAX_CORRECTION_PPM,
                "limiter violated at quantum {quantum}: {}",
                c.ppm()
            );
            let error = g.stats().error_ns_last.must();
            if disturbed {
                if error.abs() <= 10_000_000 {
                    returned_at.get_or_insert(plant.now_ns);
                } else if let Some(returned) = returned_at {
                    assert!(
                        error.abs() <= 10_000_000,
                        "left the band at t={}s (e={error} ns), returned at {returned}",
                        plant.now_ns / 1_000_000_000
                    );
                }
            }
        }
        let returned = returned_at.must();
        let disturbance_t = 60_000_000_000_u64;
        assert!(
            returned - disturbance_t <= 45_000_000_000,
            "returned {returned} ns after disturbance {disturbance_t}"
        );
        // The steady state: the correction carries the device's own
        // drift sign and magnitude order (the integral-in-the-loop
        // property — exact convergence is asymptotic with W_c = 20 s
        // and the post-disturbance walk-back is step-limited, so the
        // 120 s horizon lands within ~70 ppm of +100), and the fill
        // error sits in the proportional band (|e| ≈ W_c·δ = 2 ms).
        let ppm = g.correction().ppm();
        assert!((50..=350).contains(&ppm), "correction {ppm} not ~+100");
        assert!(g.stats().error_ns_last.must().abs() <= 5_000_000);
        // The limiter never bites inside the correctable range: the
        // undisturbed regime (error ≈ −W_c·δ = −2 ms → desired ≈
        // +100 ppm) clamps zero times. The −25 ms disturbance's
        // transient (desired ≈ +1350 ppm) legitimately saturates —
        // that is the limiter doing its job, not a violation — so it
        // is excluded from the invariant and bounded by the
        // per-quantum limiter assert above.
        assert_eq!(
            clamped_before_disturbance, 0,
            "clamped inside the correctable range before the disturbance"
        );
        // Zero underruns / zero floods after t = 10 s: the fill stays
        // strictly positive and inside the acceptance-band ceiling.
        assert!(g.stats().fill_ns_min.must() > 0, "underrun");
        assert!(
            g.stats().fill_ns_max.must() <= 150_000_000,
            "flood: {}",
            g.stats().fill_ns_max.must()
        );
    }

    #[test]
    fn s2_fixed_minus_100ppm_device_converges_without_flooding() {
        // Scenario S2: −100 ppm flooding device (symmetric). Pass: as
        // S1, plus the fill never exceeds the acceptance band's upper
        // bound (150 ms) after convergence.
        let mut g = governor();
        let mut plant = Plant::new(-100);
        let mut converged_at: Option<u64> = None;
        for _ in 0..(120 * 30) {
            let c = plant.step(&mut g);
            assert!(c.ppm().abs() <= MAX_CORRECTION_PPM);
            let error = g.stats().error_ns_last.must();
            if error.abs() <= 10_000_000 {
                converged_at.get_or_insert(plant.now_ns);
            }
        }
        let converged = converged_at.must();
        assert!(converged <= 45_000_000_000, "converged at {converged}");
        assert!(
            (g.correction().ppm() + 100).abs() <= 25,
            "{}",
            g.correction().ppm()
        );
        let fill = g.stats().fill_ns_last.must();
        assert!(
            fill <= 150_000_000,
            "fill {fill} ns left the acceptance band"
        );
    }

    #[test]
    fn s3_at_the_bound_and_adversarially_beyond() {
        // Scenario S3, first half: δ = +1000 ppm (exactly at the
        // limiter). Converges at-limit: correction pinned at the
        // bound, no oscillation (successive corrections differ by at
        // most one step-limiter increment), steady fill error
        // −W_c·δ = −20 ms.
        let mut g = governor();
        let mut plant = Plant::new(1000);
        let mut last_ppm = 0;
        let mut converged_at: Option<u64> = None;
        for _ in 0..(120 * 30) {
            let c = plant.step(&mut g);
            assert!(c.ppm() <= MAX_CORRECTION_PPM);
            assert!(
                (c.ppm() - last_ppm).abs() <= STEP_LIMIT_PPM,
                "stepped {} → {}",
                last_ppm,
                c.ppm()
            );
            last_ppm = c.ppm();
            let error = g.stats().error_ns_last.must();
            if error.abs() <= 10_000_000 {
                converged_at.get_or_insert(plant.now_ns);
            }
        }
        // At the bound the proportional offset is −20 ms: the 10 ms
        // band is NOT reachable by design (the limiter is the
        // invariant); convergence here means "at the limit value,
        // stable, within the acceptance band" (fill ≥ 10 ms). The
        // approach is asymptotic (τ = W_c), so 120 s lands within
        // ~2 ppm of the bound — and the clamp never bites for δ
        // exactly at the bound (the desired reaches 1000 only in the
        // limit); it bites beyond (the adversarial half below).
        assert!(
            g.correction().ppm() >= 990,
            "at-limit: {}",
            g.correction().ppm()
        );
        assert_eq!(g.stats().clamped, 0, "δ at the bound never trips the clamp");
        let fill = g.stats().fill_ns_last.must();
        assert!((10_000_000..=150_000_000).contains(&fill), "fill {fill}");
        let _ = converged_at;

        // Second half: δ = +5000 ppm (adversarial, beyond). The net
        // residual fill slope is exactly (d − 1000) ppm = −4 ms/s
        // (the closed-form first-order check), measured after the
        // correction pins and strictly before the ring empties (the
        // pure model reaches 0 at ≈ 9.2 s: 40 ms − 5 ms/s + the
        // +1000 ppm fight-back).
        let mut g = DriftGovernor::new(TARGET_NS as u64);
        let mut plant = Plant::new(5000);
        let mut prev_fill = TARGET_NS;
        let mut slope_per_quantum: Option<i64> = None;
        for quantum in 0..(9 * 30) {
            plant.step(&mut g);
            assert!(
                plant.fill_ns > 0,
                "model floor hit at t={}s — widen the window",
                plant.now_ns / 1_000_000_000
            );
            if (6..8).contains(&(plant.now_ns / 1_000_000_000)) {
                if let Some(pq) = slope_per_quantum {
                    assert_eq!(
                        prev_fill - plant.fill_ns,
                        pq,
                        "slope changed while pinned at quantum {quantum}"
                    );
                } else {
                    slope_per_quantum = Some(prev_fill - plant.fill_ns);
                }
            }
            prev_fill = plant.fill_ns;
        }
        assert_eq!(g.correction().ppm(), MAX_CORRECTION_PPM);
        assert!(
            g.stats().clamped > 0,
            "beyond the bound the clamp must bite"
        );
        // The fill DROP per second is exactly (d − 1000) ppm = 4 ms/s
        // (the closed-form first-order check; the slope itself — d
        // fill/dt — is −4 ms/s).
        let per_second = slope_per_quantum.must() * 30;
        assert!(
            (per_second - 4_000_000_i64).abs() <= 200_000,
            "residual drop {per_second} ns/s ≠ 4 ms/s"
        );
    }

    #[test]
    fn s5_fill_jitter_does_not_amplify() {
        // Scenario S5 (audio half): ±5 ms fill noise per quantum. The
        // correction stays within ±150 ppm (no jitter amplification —
        // the EMA in front of the control law attenuates the noise).
        let mut g = governor();
        let mut seed = 0x0085_3c49_e674_fead_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut plant = Plant::new(0);
        for quantum in 0..(60 * 30) {
            let noise = ((next() % 10_001) as i64) - 5_000; // ±5 ms
            let fill = (plant.fill_ns + noise * 1000).max(0);
            let _ = g.observe(plant.now_ns, Some(fill.unsigned_abs()));
            let c = g.correction();
            assert!(
                c.ppm().abs() <= 150,
                "jitter amplified at quantum {quantum}: {}",
                c.ppm()
            );
            plant.now_ns += plant.quantum_ns;
        }
        assert_eq!(g.stats().clamped, 0);
    }

    #[test]
    fn s6_rate_composition_converges_at_other_geometry() {
        // Scenario S6 (audio half): a 48 kHz device with a 1024-frame
        // quantum (21.33 ms — a different rate AND a different quantum
        // geometry than S1's 44.1 kHz/1470) under +100 ppm drift
        // converges exactly as S1. The correction composes with the
        // session rate by construction: the micro-resampler has NO
        // rate input (its ratio is `out = in × (1e6 + ppm)/1e6` over
        // device samples — rate-blind), and the clock is never
        // drift-corrected (doc 03 §4.7 e: neither absorber is the
        // clock; the 5/4 clock-rate composition is the clock mapping's
        // own multiplication, exercised in the apps/cli S8 loop).
        let mut g = governor();
        let mut plant = Plant {
            drift_ppm: 100,
            // A real displacement to converge from: 15 ms under
            // target (outside the 10 ms convergence band, inside the
            // correctable range — desired +750 ppm < the ±1000 ppm
            // bound, so the limiter never bites), with the +100 ppm
            // device draining further until the correction catches
            // up.
            fill_ns: TARGET_NS - 15_000_000,
            now_ns: 0,
            quantum_ns: 1024 * 1_000_000_000 / 48_000,
        };
        let mut converged_at: Option<u64> = None;
        for _ in 0..(120 * (48_000 / 1024) + 1) {
            let c = plant.step(&mut g);
            assert!(c.ppm().abs() <= MAX_CORRECTION_PPM);
            let error = g.stats().error_ns_last.must();
            if error.abs() <= 10_000_000 {
                converged_at.get_or_insert(plant.now_ns);
            }
        }
        let converged = converged_at.must();
        assert!(
            converged >= 1_000_000_000,
            "converged at {converged}: the displacement must be walked back, not assumed"
        );
        assert!(converged <= 45_000_000_000, "converged at {converged}");
        assert!((50..=350).contains(&g.correction().ppm()));
        assert!(g.stats().fill_ns_min.must() > 0, "underrun");
        assert!(g.stats().fill_ns_max.must() <= 150_000_000, "flood");
        assert_eq!(g.stats().clamped, 0);
    }

    #[test]
    fn frozen_time_source_never_re_evaluates() {
        // The S7 frozen-window property: a device pulling nothing
        // (paused / underrun-held) freezes the window — zero updates,
        // deterministically (the ProgressWindow property, mirrored).
        let mut g = governor();
        let mut plant = Plant::new(400);
        for _ in 0..120 {
            plant.step(&mut g);
        }
        let updates_before = g.stats().updates;
        let correction_before = g.correction();
        // Frozen device time: a thousand observes, no evaluation, the
        // correction holds (never winds down — R2's mitigation).
        for _ in 0..1000 {
            let c = g.observe(plant.now_ns, Some(plant.fill_ns.unsigned_abs()));
            assert_eq!(c, correction_before);
        }
        assert_eq!(g.stats().updates, updates_before);
    }

    #[test]
    fn reset_bounds_the_post_regime_transient() {
        // The S7 criterion: after an epoch-bump reset, the transient
        // is at most one step-limiter increment.
        let mut g = governor();
        let mut plant = Plant::new(800);
        for _ in 0..150 {
            plant.step(&mut g);
        }
        g.reset();
        assert_eq!(g.correction(), DriftCorrection::ZERO);
        assert_eq!(g.stats().resets, 1);
        // First observation after reset: at most +250 ppm.
        let c = g.observe(1, Some(TARGET_NS as u64));
        assert!(c.ppm() <= STEP_LIMIT_PPM, "{}", c.ppm());
    }

    #[test]
    fn step_limiter_is_counted_and_enforced() {
        // A 150 ms fill error wants −7500 ppm; `for_divergence`
        // clamps the *desired* to −1000 (the limiter is the
        // invariant), and the step limiter then bounds the *approach*:
        // −250 per window, so the bound is reached on the fourth
        // evaluation and held (never overshot).
        let mut g = governor();
        let mut now = 0_u64;
        let mut last = 0_i64;
        for second in 0..10_i64 {
            let c = g.observe(now, Some(((TARGET_MS + 150) * 1_000_000) as u64));
            assert!(
                (c.ppm() - last).abs() <= STEP_LIMIT_PPM,
                "second {second}: {} → {}",
                last,
                c.ppm()
            );
            last = c.ppm();
            now += CORRECTION_WINDOW_NS;
        }
        assert_eq!(last, -MAX_CORRECTION_PPM, "reached the bound and held");
        assert!(g.stats().step_limited >= 3, "{}", g.stats().step_limited);
    }
}
