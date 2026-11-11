//! The sample-counter clock model (card 7.H3, doc 09 §5.2) — the
//! web lane's time arithmetic: the played-instants → media-time
//! mapping, the wallclock mapping, the drift policy, and the
//! resample policy.
//!
//! # What maps where (doc 09 §5.2)
//!
//! - **Audio-master**: the media clock's anchor comes from the
//!   worklet's played-sample counter — "audio-master on web, exactly
//!   the native truth source (doc 03 §4.7)". The mapping is exact
//!   integer arithmetic ([`instants_to_ns`]; doc 01 §3.1's
//!   no-float-control-path discipline — the counter and the rates
//!   are integers, so the positions are too).
//! - **Wall-clock** (`AudioContext.currentTime`,
//!   `performance.now()`) "is only the fallback when no audio track
//!   exists": [`WorkletClock::wallclock_ns`] maps context seconds to
//!   media ns through the same anchor pair. Context seconds are an
//!   f64 *observation* (that is what the API reports); they convert
//!   to ns once, at the boundary — never accumulated.
//! - **Latency**: `AudioSink::latency()` maps to
//!   `AudioContext.baseLatency + outputLatency` where exposed
//!   (`[VERIFY per-browser outputLatency support]` — doc 09 §5.2);
//!   [`AudioLatency`] is that model, in frames and ns.
//! - **Drift**: the anchor-refresh policy — a divergence between
//!   the counter-derived wallclock expectation and the observed
//!   context time beyond [`DriftPolicy::max_drift_ns`] re-anchors
//!   the pair. The MEDIA position never jumps on a refresh (the
//!   counter is the master); the refresh re-zeros the wallclock
//!   comparison and counts itself (the drift observable).
//! - **Resample**: the `AudioContext` runs at the device rate (48
//!   kHz typical), the media at its own rate; the conversion stage
//!   resamples **once** (doc 03 §4.8 — no second hidden resampler).
//!   [`Resampler`] is that policy's arithmetic: an exact rational
//!   `out = in × num/den` with the remainder carried across chunks
//!   (the fractional accumulator — no cumulative rounding drift).
//!
//! # Evidence labels
//!
//! Every number below is proven by this crate's native tests
//! (**simulation** — real arithmetic, no browser, no real audio
//! device in this sandbox). The `AudioContext`-side facts
//! (`currentTime` semantics, `baseLatency`/`outputLatency`
//! availability and units) are `[VERIFY]` per doc 09 §8's matrix
//! (card 7.I1).

/// The canonical nanosecond base (mirrored from the present lane —
/// one import per module keeps the reads local; the mapping itself
/// is the engine-wide constant).
pub const NS_PER_SECOND: u64 = 1_000_000_000;

/// Played instants → nanoseconds at `rate_hz` (exact `u128`
/// arithmetic, saturating at `u64::MAX` — the audio-master mapping;
/// one instant = one frame across all channels, the
/// `SampleCounterSource` unit).
#[must_use]
pub fn instants_to_ns(instants: u64, rate_hz: u32) -> u64 {
    if rate_hz == 0 {
        return 0;
    }
    u64::try_from((instants as u128).saturating_mul(NS_PER_SECOND as u128) / u128::from(rate_hz))
        .unwrap_or(u64::MAX)
}

/// One `(context_time, played)` observation pair — the wallclock
/// anchor. Both halves were observed TOGETHER (the page samples
/// `AudioContext.currentTime` and the worklet's counter report in
/// one event; pairing samples from different instants would bake
/// the sampling skew into every later mapping).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallclockAnchor {
    /// `AudioContext.currentTime` (seconds) at the observation.
    pub context_s: f64,
    /// The played-sample counter (instants) at the observation.
    pub played: u64,
}

impl WallclockAnchor {
    /// The anchor's context seconds as ns (the one-time boundary
    /// conversion — the f64 observation becomes an integer ns count
    /// here and never accumulates as a float).
    #[must_use]
    pub fn context_ns(self) -> i64 {
        (self.context_s * NS_PER_SECOND as f64).round() as i64
    }
}

/// `AudioContext.baseLatency + outputLatency` (seconds; the API
/// reports doubles — `[VERIFY per-browser outputLatency support]`,
/// doc 09 §5.2). The audible position trails the played counter by
/// this much: what the listener hears is `played − latency_frames`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioLatency {
    /// `AudioContext.baseLatency` (seconds).
    pub base_s: f64,
    /// `AudioContext.outputLatency` (seconds); `0.0` where the
    /// browser does not expose it (the honest floor).
    pub output_s: f64,
}

impl AudioLatency {
    /// The total latency in seconds.
    #[must_use]
    pub fn total_s(self) -> f64 {
        self.base_s + self.output_s
    }

    /// The total latency in frames at `rate_hz` (rounded — the
    /// API's seconds do not carry a whole-frame promise).
    #[must_use]
    pub fn latency_frames(self, rate_hz: u32) -> u64 {
        (self.total_s() * f64::from(rate_hz)).round().max(0.0) as u64
    }

    /// The total latency in nanoseconds.
    #[must_use]
    pub fn latency_ns(self) -> u64 {
        (self.total_s() * NS_PER_SECOND as f64).round().max(0.0) as u64
    }
}

/// The drift policy: the anchor-refresh bound. A divergence between
/// the counter-derived wallclock expectation and the observed
/// context time beyond [`DriftPolicy::max_drift_ns`] re-anchors the
/// pair (the same anchor-refresh policy as native, doc 09 §5.2 →
/// doc 03 §4.7).
///
/// The default bound is 5 ms — well inside the ~40 ms A/V
/// perception bound, generous against scheduling jitter, and far
/// below the queue's extent. `[PROPOSAL — number; the invariant is
/// the refresh itself, not the value.]`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriftPolicy {
    /// The refresh bound (ns).
    pub max_drift_ns: u64,
}

impl DriftPolicy {
    /// The default policy (5 ms).
    pub const DEFAULT: DriftPolicy = DriftPolicy {
        max_drift_ns: 5_000_000,
    };

    /// Whether a divergence of `divergence_ns` (observed context
    /// minus counter-derived expectation) crosses the bound.
    #[must_use]
    pub fn should_refresh(&self, divergence_ns: i64) -> bool {
        divergence_ns.unsigned_abs() > self.max_drift_ns
    }
}

impl Default for DriftPolicy {
    fn default() -> Self {
        DriftPolicy::DEFAULT
    }
}

/// One wallclock observation's outcome (the drift evidence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    /// The audio-master media position at the observed counter (ns).
    pub position_ns: i64,
    /// The observed context time minus the counter-derived
    /// expectation (ns; positive = the wallclock ran ahead).
    pub divergence_ns: i64,
    /// Whether the anchor was refreshed by this observation.
    pub refreshed: bool,
}

/// The worklet clock: the anchor pair plus the drift policy, serving
/// the audio-master position (from the counter) and the wallclock
/// fallback (from context time) off ONE anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkletClock {
    rate_hz: u32,
    anchor: WallclockAnchor,
    /// The media position (ns) the anchor's counter reading maps to.
    anchor_media_ns: i64,
    policy: DriftPolicy,
    refreshes: u64,
}

impl WorkletClock {
    /// A clock anchored at `(anchor, anchor_media_ns)`.
    #[must_use]
    pub fn new(
        rate_hz: u32,
        anchor: WallclockAnchor,
        anchor_media_ns: i64,
        policy: DriftPolicy,
    ) -> WorkletClock {
        WorkletClock {
            rate_hz: rate_hz.max(1),
            anchor,
            anchor_media_ns,
            policy,
            refreshes: 0,
        }
    }

    /// The audio-master position for a counter reading (ns) — the
    /// exact integer mapping; the media truth.
    #[must_use]
    pub fn position_ns(&self, played: u64) -> i64 {
        let elapsed = played.saturating_sub(self.anchor.played);
        let delta = instants_to_ns(elapsed, self.rate_hz) as i64;
        // A played counter BELOW the anchor is a stale report, not a
        // rewind: clamp to the anchor (monotonic within the epoch —
        // doc 03 §4.7's rule, mirrored).
        self.anchor_media_ns.saturating_add(delta)
    }

    /// The wallclock fallback position for a context reading (ns) —
    /// the no-audio-track lane's time source (doc 09 §5.2).
    #[must_use]
    pub fn wallclock_ns(&self, context_s: f64) -> i64 {
        let delta_ns = (context_s * NS_PER_SECOND as f64).round() as i64 - self.anchor.context_ns();
        self.anchor_media_ns.saturating_add(delta_ns)
    }

    /// Observe one `(context_time, played)` pair: answer the
    /// audio-master position, the wallclock divergence, and whether
    /// the drift policy refreshed the anchor (the media position
    /// NEVER jumps on a refresh — the counter is the master; only
    /// the wallclock comparison re-zeros, and the refresh counts).
    pub fn observe(&mut self, context_s: f64, played: u64) -> Observation {
        let position_ns = self.position_ns(played);
        // The counter-derived context expectation (f64 seconds is
        // the observation side's unit; the divergence lands in ns).
        let elapsed_instants = played.saturating_sub(self.anchor.played);
        let expected_s =
            self.anchor.context_s + (elapsed_instants as f64) / f64::from(self.rate_hz);
        let divergence_ns = ((context_s - expected_s) * NS_PER_SECOND as f64).round() as i64;
        let refreshed = self.policy.should_refresh(divergence_ns);
        if refreshed {
            self.anchor = WallclockAnchor { context_s, played };
            self.anchor_media_ns = position_ns;
            self.refreshes = self.refreshes.saturating_add(1);
        }
        Observation {
            position_ns,
            divergence_ns,
            refreshed,
        }
    }

    /// The anchor-refresh count (the drift evidence).
    #[must_use]
    pub fn refreshes(&self) -> u64 {
        self.refreshes
    }

    /// The rate the counter ticks at.
    #[must_use]
    pub fn rate_hz(&self) -> u32 {
        self.rate_hz
    }
}

/// The resample policy's arithmetic: an exact rational
/// `out = in × num/den` with the remainder carried across chunks
/// (the fractional accumulator — no cumulative rounding drift, no
/// floats; the twin of audio-engine's `MicroResampler` phase rule).
///
/// `for_rates` reduces the ratio and answers `None` for the bypass
/// (equal rates — no conversion, doc 03 §4.8's "resamples once"
/// means at most one, and identity is zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resampler {
    num: u32,
    den: u32,
    /// The carried remainder, always `< den`.
    acc: u32,
}

impl Resampler {
    /// The conversion for `media_hz → device_hz`, or `None` when the
    /// rates are equal (the bypass). A zero rate is refused `None`
    /// too (a zero rate cannot be interpreted as time — the same
    /// typed refusal the counter makes).
    #[must_use]
    pub fn for_rates(media_hz: u32, device_hz: u32) -> Option<Resampler> {
        if media_hz == 0 || device_hz == 0 || media_hz == device_hz {
            return None;
        }
        let gcd = greatest_common_divisor(media_hz, device_hz);
        Some(Resampler {
            num: device_hz / gcd,
            den: media_hz / gcd,
            acc: 0,
        })
    }

    /// The reduced ratio `(num, den)`.
    #[must_use]
    pub fn ratio(&self) -> (u32, u32) {
        (self.num, self.den)
    }

    /// The carried remainder (the accumulator's current value —
    /// test observability).
    #[must_use]
    pub fn remainder(&self) -> u32 {
        self.acc
    }

    /// Output frames for one input chunk (the accumulator carries
    /// the fraction; consecutive chunks compose exactly —
    /// `output_frames(a) + output_frames(b) == output_frames_total(a+b)`).
    #[must_use]
    pub fn output_frames(&mut self, input_frames: u64) -> u64 {
        let total =
            (input_frames.saturating_mul(u64::from(self.num))).saturating_add(u64::from(self.acc));
        let out = total / u64::from(self.den);
        self.acc = (total % u64::from(self.den)) as u32;
        out
    }
}

/// Euclid's gcd (std has none for u32; five lines, no deps).
const fn greatest_common_divisor(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let rest = a % b;
        a = b;
        b = rest;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The audio-master mapping is exact: 48 kHz · 480 000 instants
    /// = exactly 10 s; 44.1 kHz lands on whole ns (the u128 math has
    /// no rounding to drift).
    #[test]
    fn the_counter_mapping_is_exact() {
        assert_eq!(instants_to_ns(480_000, 48_000), 10_000_000_000);
        assert_eq!(instants_to_ns(44_100, 44_100), NS_PER_SECOND);
        assert_eq!(instants_to_ns(1, 48_000), 20_833, "1/48000 s in ns");
        assert_eq!(instants_to_ns(0, 48_000), 0);
        assert_eq!(instants_to_ns(1_000, 0), 0, "a zero rate is unusable");
        // The clock's epoch arithmetic: anchor + elapsed, clamped
        // below the anchor (a stale report never rewinds).
        let clock = WorkletClock::new(
            48_000,
            WallclockAnchor {
                context_s: 1.0,
                played: 48_000,
            },
            2_000_000_000,
            DriftPolicy::DEFAULT,
        );
        assert_eq!(
            clock.position_ns(48_000),
            2_000_000_000,
            "the anchor itself"
        );
        assert_eq!(
            clock.position_ns(96_000),
            2_000_000_000 + 1_000_000_000,
            "one second past the anchor"
        );
        assert_eq!(
            clock.position_ns(24_000),
            2_000_000_000,
            "a stale report clamps to the anchor"
        );
    }

    /// The wallclock fallback: context seconds map through the same
    /// anchor (the no-audio lane's time source).
    #[test]
    fn the_wallclock_fallback_maps_through_the_anchor() {
        let clock = WorkletClock::new(
            48_000,
            WallclockAnchor {
                context_s: 10.0,
                played: 480_000,
            },
            5_000_000_000,
            DriftPolicy::DEFAULT,
        );
        assert_eq!(clock.wallclock_ns(10.0), 5_000_000_000);
        assert_eq!(clock.wallclock_ns(11.5), 6_500_000_000);
        assert_eq!(clock.wallclock_ns(9.5), 4_500_000_000);
    }

    /// The latency model: base + output in frames and ns.
    #[test]
    fn the_latency_model_adds_and_converts() {
        let latency = AudioLatency {
            base_s: 0.005,
            output_s: 0.010,
        };
        assert!((latency.total_s() - 0.015).abs() < 1e-12);
        assert_eq!(latency.latency_frames(48_000), 720, "15 ms at 48 kHz");
        assert_eq!(latency.latency_ns(), 15_000_000);
        // The honest floor where outputLatency is absent.
        let partial = AudioLatency {
            base_s: 0.005,
            output_s: 0.0,
        };
        assert_eq!(partial.latency_frames(48_000), 240);
        assert!(partial.latency_ns() < latency.latency_ns());
    }

    /// The drift simulation (native-cfg, real arithmetic, labeled
    /// simulation — no browser): a true hardware rate 2 Hz under
    /// nominal (47 998 vs 48 000, a −41.7 ppm skew) walks the
    /// divergence at ~41 667 ns/s; the 5 ms bound refreshes every
    /// ~120 s, the media position NEVER jumps on a refresh, and no
    /// refresh fires below the bound.
    #[test]
    fn drift_simulation_refreshes_at_the_bound_never_earlier() {
        let nominal = 48_000_u32;
        let mut clock = WorkletClock::new(
            nominal,
            WallclockAnchor {
                context_s: 0.0,
                played: 0,
            },
            0,
            DriftPolicy::DEFAULT,
        );
        let mut played = 0_u64;
        let mut refresh_seconds = Vec::new();
        for second in 1..=300_u64 {
            // One wallclock second passed: the true device consumed
            // 47 998 frames (an integral rate — the simulation's
            // chosen skew; a fractional rate would accumulate the
            // fraction into the integral counter the same way).
            played += 47_998;
            let observation = clock.observe(second as f64, played);
            // The audio-master position: the counter's mapping
            // (≤1 ns of epoch-re-anchor flooring, never a jump).
            let expected = instants_to_ns(played, nominal) as i64;
            assert!(
                (observation.position_ns - expected).abs() <= 1,
                "position {} vs counter {} at {second}s",
                observation.position_ns,
                expected
            );
            if observation.refreshed {
                refresh_seconds.push(second);
                // At a refresh the divergence was beyond the bound…
                assert!(
                    observation.divergence_ns.unsigned_abs() > 5_000_000,
                    "refresh at {second}s with divergence {}",
                    observation.divergence_ns
                );
                // …and the position did not jump (the re-anchor maps
                // the same instant to the same ns).
            } else {
                assert!(
                    observation.divergence_ns.unsigned_abs() <= 5_000_000,
                    "no refresh at {second}s but divergence {}",
                    observation.divergence_ns
                );
            }
        }
        // 300 s × 41 667 ns/s ≈ 12.5 ms of total divergence: the
        // first refresh lands near 120 s, the second near 240 s.
        assert_eq!(refresh_seconds.len(), 2, "refreshes at {refresh_seconds:?}");
        let first = refresh_seconds[0];
        assert!((110..=130).contains(&first), "first at {first}s");
        let second = refresh_seconds[1];
        assert!(
            (110..=130).contains(&(second - first)),
            "the second cycle is another ~120 s ({first} → {second})"
        );
    }

    /// A zero-skew simulation never refreshes (the bound is never
    /// crossed by an exact clock).
    #[test]
    fn zero_drift_never_refreshes() {
        let mut clock = WorkletClock::new(
            48_000,
            WallclockAnchor {
                context_s: 0.0,
                played: 0,
            },
            0,
            DriftPolicy::DEFAULT,
        );
        let mut played = 0_u64;
        for second in 1..=600_u64 {
            played += 48_000;
            let observation = clock.observe(second as f64, played);
            assert!(!observation.refreshed, "no skew, no refresh");
            assert_eq!(observation.divergence_ns, 0);
        }
        assert_eq!(clock.refreshes(), 0);
    }

    /// The resample policy: the 44.1 → 48 kHz classic is exactly
    /// 160/147; arbitrary chunk sizes compose with no cumulative
    /// drift (the accumulator carries the fraction).
    #[test]
    fn the_resampler_is_exact_rational_arithmetic() {
        // Equal rates are the bypass (no conversion at all).
        assert!(Resampler::for_rates(48_000, 48_000).is_none());
        assert!(Resampler::for_rates(0, 48_000).is_none(), "zero is refused");
        let mut resampler = Resampler::for_rates(44_100, 48_000).must();
        assert_eq!(resampler.ratio(), (160, 147));
        // The classic quantum: 147 in → 160 out, remainder zero.
        assert_eq!(resampler.output_frames(147), 160);
        assert_eq!(resampler.remainder(), 0);
        // Arbitrary chunks: 100 then 47 compose to the same total.
        let mut split = Resampler::for_rates(44_100, 48_000).must();
        assert_eq!(split.output_frames(100), 108);
        assert_eq!(split.remainder(), 124, "16000 = 108×147 + 124");
        assert_eq!(split.output_frames(47), 52);
        assert_eq!(split.remainder(), 0, "7644 = 52×147 exactly");
        // 22050 → 48000 reduces to 320/147.
        let odd = Resampler::for_rates(22_050, 48_000).must();
        assert_eq!(odd.ratio(), (320, 147));
    }
}
