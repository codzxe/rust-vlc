//! A/V drift estimation and band policy (card 3.B1; doc 03 §4.7 e /
//! §4.10 e — the W1-18 design's pure L2 core).
//!
//! The measurement half of the 3.B1 audio-master drift loop. Doc 03
//! §4.7(e) assigns drift to two absorbers, *neither of which is the
//! clock*; both absorbers need the same two pure primitives, which
//! live here because media-clock is the only crate below both engines
//! (L2, media-types only — boundaries.toml's audio-engine edge note):
//!
//! - [`DriftEstimator`]: an integer EMA over per-sample error values,
//!   plus a windowed slope over an **injected** time source. The video
//!   scheduler feeds it per-present presentation errors
//!   `e_v = pts − now` (no time source — the EMA only); the audio
//!   governor feeds it per-quantum fill errors `e = fill − target`
//!   (windowed — the slope is the observed device-vs-nominal rate
//!   mismatch, an observability number, not a control input).
//! - [`SyncBands`]: the Schmitt-trigger hold/correct band classifier
//!   of the video drop/repeat ladder (doc 03 §4.10 e + the W1-18
//!   hysteresis design): enter a correction band at `|ê| > H`
//!   sustained `D` ticks, exit at `|ê| ≤ H/2` sustained `D` ticks.
//! - [`RateWindow`]: a fixed coalescing window over an injected
//!   monotonic source — the ≤1 Hz correction re-evaluation carrier
//!   (3.B4's `ProgressWindow` semantics, re-implemented here beside
//!   the estimator because playback-engine is L4 and cannot be
//!   imported downward; deliberate, not duplication-by-oversight).
//!
//! # Discipline
//!
//! - **Integer arithmetic throughout** on the control path (doc 01
//!   §3.1 — no floats, no cumulative rounding drift). The EMA is the
//!   dyadic `ema = (x + (2^k − 1)·ema) >> k`; the slope is exact
//!   `i128` ns-per-ns differencing exposed in milli-ppm.
//! - **Injected time, never ambient** (the events.rs injection
//!   doctrine): every window takes `now_ns` from its caller. A frozen
//!   source yields a frozen window — zero updates, deterministically
//!   (the `ProgressWindow` frozen-source property, mirrored and
//!   tested below).
//! - **Reset semantics** (epoch / generation / pause) are the
//!   *callers'* decisions — the information lives at the call sites
//!   (the scheduler sees `ClockSnapshot`, the governor sees the
//!   wiring). This module provides [`DriftEstimator::reset`] and stays
//!   oblivious to why.
//! - **Bounded for any input** (D-1/D-2): fixed-size state (a handful
//!   of integers), `Copy`-cheap, zero allocation, saturating `i128`
//!   intermediates — an i64 extreme sample can never overflow the EMA
//!   (a convex combination stays within the input range).

/// The dyadic EMA weight `k` default `[PROPOSAL per W1-18 §2.1]`:
/// `ema = (x + (2^k − 1)·ema) >> k` — 3 ⇒ a new sample carries 1/8,
/// the effective averaging window ≈ 8 samples (≈267 ms of 30 fps
/// presents, ≈267 ms of 30 Hz fill samples). Integer-exact, no floats.
pub const DEFAULT_EMA_SHIFT: u32 = 3;

/// The slope window default `[PROPOSAL per W1-18 §2.2]`: 2 s of
/// injected time — the Δe/Δt basis for the observed rate mismatch.
pub const DEFAULT_SLOPE_WINDOW_NS: u64 = 2_000_000_000;

/// One nanosecond, as an `i128` factor for slope math.
const NS: i128 = 1;

/// A pure drift-error estimator: integer EMA over error samples, plus
/// a windowed slope over an injected monotonic time source.
///
/// Two cadences, one state (the W1-18 design's single estimator type):
///
/// - [`DriftEstimator::observe_sample`] — the per-present video
///   cadence (the scheduler has no time source; the EMA is the filter).
/// - [`DriftEstimator::observe_windowed`] — the per-quantum audio
///   cadence (same EMA, plus the Δe/Δt slope over the window).
///
/// The EMA of a convex dyadic update is bounded by the input range:
/// `min(x_i) ≤ ema ≤ max(x_i)` for any input sequence — no overflow,
/// no runaway, saturating only at the i64 extremes themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriftEstimator {
    /// Dyadic weight `k` (weight of history `2^k − 1`).
    shift: u32,
    /// The filtered error, ns. Saturating i64 (a convex combination
    /// of i64 inputs needs no wider exposure).
    ema: i64,
    /// Whether at least one sample has been taken since the last
    /// [`DriftEstimator::reset`].
    primed: bool,
    /// The slope window length (injected-ns).
    window_ns: u64,
    /// The window's opening (injected-ns); `None` until the first
    /// windowed sample.
    opened_ns: Option<u64>,
    /// The error at the window's opening (for Δe).
    opened_error: i64,
    /// The slope EMA, in **milli-ppm** (ppm × 1000) — integer
    /// resolution enough for observability without floats.
    slope_mppm: i64,
    /// Whether a slope has been measured since the last reset.
    slope_primed: bool,
    /// Lifetime sample count (monotonic across windows).
    samples: u64,
}

impl DriftEstimator {
    /// The default estimator (W1-18's `[PROPOSAL]` numbers: k = 3,
    /// slope window 2 s).
    #[must_use]
    pub fn new() -> DriftEstimator {
        DriftEstimator::for_shift_and_window(DEFAULT_EMA_SHIFT, DEFAULT_SLOPE_WINDOW_NS)
    }

    /// The explicit constructor. A zero shift is legal (EMA = the last
    /// sample — no filtering); a zero slope window disables slope
    /// measurement (the window never elapses).
    #[must_use]
    pub fn for_shift_and_window(shift: u32, window_ns: u64) -> DriftEstimator {
        DriftEstimator {
            shift,
            ema: 0,
            primed: false,
            window_ns,
            opened_ns: None,
            opened_error: 0,
            slope_mppm: 0,
            slope_primed: false,
            samples: 0,
        }
    }

    /// Observe one error sample on the **per-present** cadence (the
    /// video side: `e_v = pts − now` at each present; no time source).
    pub fn observe_sample(&mut self, error_ns: i64) {
        self.ema_step(error_ns);
    }

    /// Observe one error sample on the **per-quantum** cadence (the
    /// audio side: `e = fill − target`), advancing the slope window on
    /// the injected time source. A frozen `now_ns` never elapses the
    /// window: the slope holds (the frozen-source property).
    pub fn observe_windowed(&mut self, now_ns: u64, error_ns: i64) {
        self.ema_step(error_ns);
        self.samples = self.samples.saturating_add(1);
        match self.opened_ns {
            None => {
                self.opened_ns = Some(now_ns);
                self.opened_error = error_ns;
            }
            Some(opened) => {
                if now_ns.saturating_sub(opened) >= self.window_ns {
                    // Δe/Δt over the elapsed window, in milli-ppm:
                    // (Δe_ns / Δt_ns) × 1e9 = ppm × 1000. Exact i128,
                    // saturating to i64 (extreme ramps are unphysical;
                    // the bound is the honest answer, never a panic).
                    let dt = i128::from(now_ns.saturating_sub(opened)).max(NS);
                    let de = i128::from(error_ns) - i128::from(self.opened_error);
                    let slope = de
                        .checked_mul(1_000_000_000)
                        .map(|scaled| scaled / dt)
                        .unwrap_or(i64::MAX as i128);
                    self.slope_mppm = saturate_i64(slope);
                    self.slope_primed = true;
                    self.opened_ns = Some(now_ns);
                    self.opened_error = error_ns;
                }
            }
        }
    }

    /// The filtered error (the EMA), or `None` before any sample.
    #[must_use]
    pub fn error_ema(&self) -> Option<i64> {
        self.primed.then_some(self.ema)
    }

    /// The last measured slope, in **milli-ppm** (ns error per ns
    /// time × 1e9), or `None` before a window has elapsed.
    #[must_use]
    pub fn slope_mppm(&self) -> Option<i64> {
        self.slope_primed.then_some(self.slope_mppm)
    }

    /// The last measured slope, rounded to whole ppm (observability).
    ///
    /// Ceiling division on signed milli-ppm, spelled out by hand: the
    /// toolchain's `i64::div_ceil` is still `int_roundings`-unstable,
    /// and the one-line identity below carries the same semantics
    /// (round toward +∞) with stable primitives.
    #[must_use]
    pub fn slope_ppm(&self) -> Option<i64> {
        self.slope_mppm().map(|mppm| {
            let quotient = mppm / 1000;
            let remainder = mppm % 1000;
            if remainder > 0 {
                quotient + 1
            } else {
                quotient
            }
        })
    }

    /// Total windowed samples since construction (monotonic; not
    /// reset — callers track resets through their own epochs).
    #[must_use]
    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// Reset the estimator: a new regime (generation / anchor-epoch /
    /// pause per the callers' policies). Both the EMA and the slope
    /// window restart from the next sample.
    pub fn reset(&mut self) {
        self.ema = 0;
        self.primed = false;
        self.opened_ns = None;
        self.opened_error = 0;
        self.slope_mppm = 0;
        self.slope_primed = false;
    }

    /// One dyadic EMA step: `ema = (x + (2^k − 1)·ema) >> k`, checked
    /// i128, saturated to i64. A convex combination of i64 values
    /// stays in the input range, so saturation is reachable only
    /// through i64-extreme inputs themselves.
    fn ema_step(&mut self, error_ns: i64) {
        if !self.primed {
            self.ema = error_ns;
            self.primed = true;
            return;
        }
        let weight = (1u128 << self.shift.min(126)).saturating_sub(1) as i128;
        let combined =
            i128::from(error_ns).saturating_add(i128::from(self.ema).saturating_mul(weight));
        self.ema = saturate_i64(combined >> self.shift);
    }
}

impl Default for DriftEstimator {
    fn default() -> DriftEstimator {
        DriftEstimator::new()
    }
}

/// Saturate an `i128` nanosecond-scale value into `i64` (the
/// media-clock position discipline: saturate, never wrap).
fn saturate_i64(value: i128) -> i64 {
    value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

// ---------------------------------------------------------------------------
// Sync bands — the Schmitt-trigger ladder classifier (video side)
// ---------------------------------------------------------------------------

/// The correction band the video ladder is in (doc 03 §4.10 e
/// actuators, the W1-18 §3.1 control law).
///
/// Sign convention (the estimator's `e_v = pts − now`): **negative =
/// presents running behind the clock** (the decoded stream cannot
/// keep up; frames wedge against the late bound), **positive =
/// presents running ahead** (the decoded stream is buffered ahead;
/// starvation gaps between bursts are bridged by repeats).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncBand {
    /// `|ê| ≤ H`: the default policy — the scheduler's own
    /// lead/threshold already absorbs the error. No adaptation.
    Hold,
    /// `ê < −H` sustained: widen the effective late threshold
    /// (tolerate bounded lateness instead of a drop firehose),
    /// bounded by the drop budget and the widening cap.
    WidenLate,
    /// `ê > +H` sustained: extend the effective repeat budget (+2) —
    /// bridge starvation gaps longer before backgrounding. Audio is
    /// never touched (doc 03 §4.10 e).
    ExtendEarly,
}

/// The Schmitt-trigger band state machine: enter a correction band at
/// `|ê| > H` sustained `D` ticks, return to [`SyncBand::Hold`] at
/// `|ê| ≤ H/2` sustained `D` ticks. The gap `(H/2, H]` is deliberate
/// hysteresis — the band in force persists through it, so a signal
/// hovering near the threshold cannot flap (W1-18 §3.1; S5's
/// no-flapping assertion is pinned against this type).
///
/// `H` defaults to half a frame duration (equal to the scheduler's
/// default `target_lead` — "the hold band is within the lead the
/// scheduler already absorbs"). Pure: a function of the filtered error
/// stream only; the *actuators* (threshold widening, repeat extension)
/// live in video-engine's `SyncPolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncBands {
    /// The half-band `H`, ns.
    half_ns: i64,
    /// The dwell `D`, in classify calls (scheduler ticks).
    dwell: u32,
    /// The band in force.
    band: SyncBand,
    /// Consecutive ticks sustained beyond the entry threshold (late
    /// side).
    late_streak: u32,
    /// Consecutive ticks sustained beyond the entry threshold (early
    /// side).
    early_streak: u32,
    /// Consecutive ticks calm (within the exit threshold).
    calm_streak: u32,
}

impl SyncBands {
    /// A band machine with half-band `half_band_ns` and dwell
    /// `dwell_ticks` `[PROPOSAL per W1-18: H = ½ frame, D = 8 ticks]`.
    #[must_use]
    pub fn new(half_band_ns: u64, dwell_ticks: u32) -> SyncBands {
        SyncBands {
            half_ns: saturate_i64(i128::from(half_band_ns)),
            dwell: dwell_ticks.max(1),
            band: SyncBand::Hold,
            late_streak: 0,
            early_streak: 0,
            calm_streak: 0,
        }
    }

    /// The band currently in force.
    #[must_use]
    pub fn band(&self) -> SyncBand {
        self.band
    }

    /// The half-band `H` in force (ns).
    #[must_use]
    pub fn half_band_ns(&self) -> i64 {
        self.half_ns
    }

    /// Classify one tick's filtered error. `None` (no sample — the
    /// clock is paused, or nothing presented) holds the current band
    /// and decays every streak to zero: absence is not evidence for
    /// either direction (a repeat already tells the sign: nothing was
    /// due).
    pub fn classify(&mut self, error_ema_ns: Option<i64>) -> SyncBand {
        let Some(e) = error_ema_ns else {
            self.late_streak = 0;
            self.early_streak = 0;
            self.calm_streak = 0;
            return self.band;
        };
        // i128 compare: |e| against H cannot overflow for i64 inputs
        // widened once.
        let magnitude = i128::from(e).abs();
        match self.band {
            SyncBand::Hold => {
                if magnitude > i128::from(self.half_ns) {
                    if e < 0 {
                        self.late_streak = self.late_streak.saturating_add(1);
                        self.early_streak = 0;
                    } else {
                        self.early_streak = self.early_streak.saturating_add(1);
                        self.late_streak = 0;
                    }
                    self.calm_streak = 0;
                } else {
                    self.late_streak = 0;
                    self.early_streak = 0;
                }
                if self.late_streak >= self.dwell {
                    self.band = SyncBand::WidenLate;
                    self.clear_streaks();
                } else if self.early_streak >= self.dwell {
                    self.band = SyncBand::ExtendEarly;
                    self.clear_streaks();
                }
            }
            SyncBand::WidenLate | SyncBand::ExtendEarly => {
                // Exit at half amplitude, sustained.
                if magnitude <= i128::from(self.half_ns) / 2 {
                    self.calm_streak = self.calm_streak.saturating_add(1);
                } else {
                    self.calm_streak = 0;
                }
                if self.calm_streak >= self.dwell {
                    self.band = SyncBand::Hold;
                    self.clear_streaks();
                }
            }
        }
        self.band
    }

    /// Reset to [`SyncBand::Hold`] with cleared streaks (a new regime:
    /// generation / anchor-epoch / snap).
    pub fn reset(&mut self) {
        self.band = SyncBand::Hold;
        self.clear_streaks();
    }

    fn clear_streaks(&mut self) {
        self.late_streak = 0;
        self.early_streak = 0;
        self.calm_streak = 0;
    }
}

// ---------------------------------------------------------------------------
// RateWindow — the ≤1 Hz re-evaluation carrier (audio side)
// ---------------------------------------------------------------------------

/// A fixed coalescing window over an **injected** time source
/// (3.B4's `ProgressWindow` semantics, re-implemented at L2 beside its
/// consumer — playback-engine cannot be imported downward): accepts
/// `now_ns`, answers whether the window has elapsed (and reopens).
///
/// The frozen-source property: a never-advancing source permits
/// exactly one `true` (the opening call), then none — deterministic
/// under paused clocks (W1-18 §2.2; pinned by test below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateWindow {
    window_ns: u64,
    opened_ns: Option<u64>,
}

impl RateWindow {
    /// A window of `window_ns` injected nanoseconds.
    #[must_use]
    pub fn new(window_ns: u64) -> RateWindow {
        RateWindow {
            window_ns,
            opened_ns: None,
        }
    }

    /// Whether a re-evaluation is permitted at `now_ns`: the first
    /// call opens the window and permits; later calls wait for it to
    /// elapse (saturating subtraction — a source that jumps backwards
    /// merely holds, never misfires).
    #[must_use]
    pub fn permits(&mut self, now_ns: u64) -> bool {
        match self.opened_ns {
            None => {
                self.opened_ns = Some(now_ns);
                true
            }
            Some(opened) => {
                if now_ns.saturating_sub(opened) >= self.window_ns {
                    self.opened_ns = Some(now_ns);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Drop the window (a regime reset): the next `permits` reopens.
    pub fn reset(&mut self) {
        self.opened_ns = None;
    }

    /// The window length in force (ns).
    #[must_use]
    pub fn window_ns(&self) -> u64 {
        self.window_ns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    // ---- DriftEstimator: boundedness + filter semantics (S10) ---------

    #[test]
    fn ema_is_primed_by_the_first_sample_and_converges() {
        let mut est = DriftEstimator::for_shift_and_window(3, 0);
        assert_eq!(est.error_ema(), None);
        est.observe_sample(1_000_000);
        assert_eq!(est.error_ema(), Some(1_000_000));
        // A constant stream converges to that constant exactly (a
        // fixed point of the dyadic update).
        for _ in 0..64 {
            est.observe_sample(1_000_000);
        }
        assert_eq!(est.error_ema(), Some(1_000_000));
    }

    #[test]
    fn ema_is_bounded_by_the_input_range_for_any_sequence() {
        // S10 property: no overflow, no runaway — a convex combination
        // stays within [min, max] of the inputs. Extremes included.
        let mut est = DriftEstimator::new();
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut min = i64::MAX;
        let mut max = i64::MIN;
        for _ in 0..4096 {
            let x = next() as i64;
            min = min.min(x);
            max = max.max(x);
            est.observe_sample(x);
            let ema = est.error_ema().must();
            assert!(ema >= min && ema <= max, "ema {ema} escaped [{min}, {max}]");
        }
        // i64 extremes: the EMA is a convex combination, so it stays
        // within [min, max] of what it has seen — saturating only at
        // the extreme itself, never panicking, never wrapping past it.
        // The dyadic floor stalls the approach to a positive fixed
        // point at up to 2^k − 1 = 7 units below `i64::MAX` (k = 3:
        // `ema' = ema` once `MAX − ema ≤ 7`, since `floor((8·MAX −
        // 7d)/8) = MAX − ceil(7d/8)` fixes at d = 7) — the exact stall
        // point, hand-verified; documented, bounded, irrelevant at ns
        // scale. The negative side has no stall: `>>` floors toward
        // −∞, so a constant `i64::MIN` stream converges exactly.
        for _ in 0..1024 {
            est.observe_sample(i64::MAX);
        }
        assert!(est.error_ema().must() >= i64::MAX - 7);
        let mut previous = i64::MAX;
        for _ in 0..64 {
            est.observe_sample(i64::MIN);
            let ema = est.error_ema().must();
            assert!(ema <= previous, "ema {ema} rose above {previous}");
            previous = ema;
        }
        // A long enough MIN stream converges to MIN exactly (fixed point).
        for _ in 0..1024 {
            est.observe_sample(i64::MIN);
        }
        assert_eq!(est.error_ema(), Some(i64::MIN));
    }

    #[test]
    fn ema_step_weight_matches_the_dyadic_formula() {
        // k = 2: ema' = (x + 3·ema) >> 2. Hand-checked ladder.
        let mut est = DriftEstimator::for_shift_and_window(2, 0);
        est.observe_sample(0);
        est.observe_sample(800);
        assert_eq!(est.error_ema(), Some(200));
        est.observe_sample(800);
        assert_eq!(est.error_ema(), Some(350));
        est.observe_sample(0);
        assert_eq!(est.error_ema(), Some(262));
    }

    #[test]
    fn windowed_slope_measures_an_exact_ramp_in_milli_ppm() {
        // e(t) = −1000 ns per injected second ⇒ de/dt = −1e-6 exactly =
        // −1 ppm = −1000 milli-ppm. Window 2 s, samples 1 s apart.
        let mut est = DriftEstimator::for_shift_and_window(3, 2_000_000_000);
        est.observe_windowed(0, 0);
        for second in 1i64..=10 {
            est.observe_windowed(u64::try_from(second).must() * 1_000_000_000, -1000 * second);
        }
        assert_eq!(est.slope_mppm(), Some(-1000));
        assert_eq!(est.slope_ppm(), Some(-1));
        // Samples are counted, monotonic.
        assert_eq!(est.samples(), 11);
    }

    #[test]
    fn frozen_source_never_elapses_the_slope_window() {
        let mut est = DriftEstimator::for_shift_and_window(3, 1_000_000);
        est.observe_windowed(5_000_000, 1_000);
        // Frozen at 5 s forever: the window never elapses, the slope
        // stays unmeasured.
        for _ in 0..100 {
            est.observe_windowed(5_000_000, 999_999);
        }
        assert_eq!(est.slope_mppm(), None);
    }

    #[test]
    fn reset_restarts_ema_and_slope_regime() {
        let mut est = DriftEstimator::new();
        est.observe_sample(500_000);
        est.observe_windowed(10_000_000_000, 500_000);
        est.reset();
        assert_eq!(est.error_ema(), None);
        assert_eq!(est.slope_mppm(), None);
        est.observe_sample(-7);
        assert_eq!(est.error_ema(), Some(-7));
    }

    // ---- SyncBands: Schmitt trigger + dwell (S10) ----------------------

    fn bands() -> SyncBands {
        SyncBands::new(16_666_667, 8)
    }

    #[test]
    fn hold_band_absorbs_errors_within_the_half_band() {
        let mut b = bands();
        for _ in 0..100 {
            assert_eq!(b.classify(Some(16_666_667)), SyncBand::Hold);
            assert_eq!(b.classify(Some(-16_666_667)), SyncBand::Hold);
        }
    }

    #[test]
    fn entry_requires_dwell_sustained_beyond_the_half_band() {
        let mut b = bands();
        // 7 sustained late ticks: not yet.
        for _ in 0..7 {
            assert_eq!(b.classify(Some(-100_000_000)), SyncBand::Hold);
        }
        // An interruption resets the streak (hysteresis discipline).
        assert_eq!(b.classify(Some(0)), SyncBand::Hold);
        for _ in 0..7 {
            assert_eq!(b.classify(Some(-100_000_000)), SyncBand::Hold);
        }
        // The 8th consecutive: enter.
        assert_eq!(b.classify(Some(-100_000_000)), SyncBand::WidenLate);
        // The early side, symmetric.
        let mut b = bands();
        for _ in 0..8 {
            let _ = b.classify(Some(100_000_000));
        }
        assert_eq!(b.band(), SyncBand::ExtendEarly);
    }

    #[test]
    fn exit_requires_half_amplitude_sustained_for_dwell() {
        let mut b = bands();
        for _ in 0..8 {
            let _ = b.classify(Some(-100_000_000));
        }
        assert_eq!(b.band(), SyncBand::WidenLate);
        // Inside the hysteresis gap (H/2, H]: the band persists.
        for _ in 0..50 {
            assert_eq!(b.classify(Some(-10_000_000)), SyncBand::WidenLate);
        }
        // At half amplitude, 7 ticks: not yet (the 8th calm tick is
        // the one that exits — symmetric with the entry test's 8th
        // sustained tick entering; the checkpoint draft asserted a
        // ninth WidenLate here, an off-by-one against its own pinned
        // entry symmetry).
        for _ in 0..7 {
            assert_eq!(b.classify(Some(-8_000_000)), SyncBand::WidenLate);
        }
        // 8th: back to hold.
        assert_eq!(b.classify(Some(-8_000_000)), SyncBand::Hold);
    }

    #[test]
    fn none_samples_hold_the_band_and_decay_streaks() {
        let mut b = bands();
        for _ in 0..7 {
            let _ = b.classify(Some(-100_000_000));
        }
        // Absence (paused clock, no present): not evidence.
        for _ in 0..10 {
            assert_eq!(b.classify(None), SyncBand::Hold);
        }
        // The entry streak was decayed: one more late tick is not enough.
        assert_eq!(b.classify(Some(-100_000_000)), SyncBand::Hold);
    }

    #[test]
    fn reset_returns_to_hold() {
        let mut b = bands();
        for _ in 0..8 {
            let _ = b.classify(Some(-100_000_000));
        }
        b.reset();
        assert_eq!(b.band(), SyncBand::Hold);
    }

    // ---- RateWindow (S10) ----------------------------------------------

    #[test]
    fn rate_window_first_call_opens_and_permits_once() {
        let mut w = RateWindow::new(1_000_000_000);
        assert!(w.permits(123));
        assert!(!w.permits(123 + 999_999_999));
        assert!(w.permits(123 + 1_000_000_000));
        assert!(!w.permits(123 + 1_999_999_999));
        assert!(w.permits(123 + 2_000_000_000));
    }

    #[test]
    fn rate_window_frozen_source_permits_exactly_one() {
        let mut w = RateWindow::new(1_000_000_000);
        assert!(w.permits(7));
        for _ in 0..1000 {
            assert!(!w.permits(7));
        }
    }

    #[test]
    fn rate_window_backwards_jump_holds_and_reset_reopens() {
        let mut w = RateWindow::new(1_000_000_000);
        assert!(w.permits(10_000_000_000));
        // A backwards (seek) jump: saturating subtraction holds.
        assert!(!w.permits(1_000_000_000));
        assert!(!w.permits(9_999_999_999));
        w.reset();
        assert!(w.permits(500));
    }
}
