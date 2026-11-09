//! The progressive adaptive buffer model (card 6.D1): watermarks,
//! stall prediction, and rebuffer metrics as one pure policy core.
//!
//! # What this module owns (and deliberately does not)
//!
//! The model is **data in, signals out** — the recovery.rs doctrine
//! from the playback engine ("policy is data, the executor owns
//! mutation"): the host that drives a network source observes the
//! buffer after every fetch/serve step
//! ([`BufferModel::observe`](BufferModel::observe)) and the model
//! answers the one question each consumer has:
//!
//! - the **watermark state machine** ([`BufferLevel`]) with
//!   hysteresis: `Low → High` requires meeting the fill watermark on
//!   **both** axes (bytes AND media duration — confidence before
//!   resuming), `High → Low` trips when **either** axis drains below
//!   its drain watermark (early warning before a hard underrun);
//! - **stall prediction**: an EWMA of the consumption rate (media
//!   time consumed per wall time) against an EWMA of the arrival
//!   rate (media time received per wall time — derived from the
//!   buffered delta plus consumption, so the host reports one
//!   tuple); when the deficit drains the runway inside the
//!   configured horizon, the model reports
//!   [`BufferSignal::StallPredicted`] with the honest ETA —
//!   *before* the queues starve, which is the whole point of
//!   prediction;
//! - **rebuffer metrics**: episode counts and durations (initial
//!   fill separated from rebuffers — VLC's accounting shape), the
//!   current episode's age, and totals; every duration comes from
//!   the caller's `now_ns` (the clock-injection doctrine — no
//!   ambient time in a decision).
//!
//! It does **not** own any I/O, thread, or queue: nothing here
//! fetches, sleeps, or allocates ([`BufferSignal`] and
//! [`BufferSnapshot`] are `Copy`; the observation path is
//! allocation-free — engineering directive D-1).
//!
//! # Time doctrine
//!
//! `now_ns` is **nanoseconds on the caller's monotonic clock** (the
//! [`crate::clock::WallClock`] port's `now_ms` × 1e6, or the
//! engine's injected source). Tests feed virtual time; the model is
//! deterministic. Rates are kept per-millisecond internally
//! (media-ns per wall-ms) as `u128` fixed-point — computed with one
//! division per *observation* (one per network fetch/serve, not per
//! frame), matching the layer's existing per-request parse math.
//!
//! # Watermark arithmetic (why both axes)
//!
//! Bytes measure the **network** contract (the window/retention
//! budgets of `ranged`/`hls`); media duration measures the
//! **playback** contract (how long the runway actually is). A stream
//! can be byte-rich but duration-poor (a 4K slab is 30 s of bytes
//! and 2 s of video) or the reverse (a tiny audio trickle). Filling
//! requires both to be satisfied; draining trips on either — the
//! conservative pair, so a `High` verdict is always safe to resume
//! on and a `Low` verdict always arrives before a hard underrun on
//! either axis.

use crate::clock::WallClock;

/// Nanoseconds per wall-clock millisecond (the rate unit's scaling).
pub const NS_PER_MS: u64 = 1_000_000;

/// The observation helper for clock-sharing hosts: nanoseconds on
/// the [`WallClock`] port (the model's `now_ns` inputs are exactly
/// this reading — one shared clock, one time base).
pub fn now_ns(clock: &dyn WallClock) -> u64 {
    clock.now_ms() * NS_PER_MS
}

/// Default fill watermark (media): 4 s of runway before `High`.
pub const DEFAULT_FILL_MEDIA_NS: u64 = 4_000_000_000;

/// Default drain watermark (media): 500 ms of runway trips `Low`.
pub const DEFAULT_DRAIN_MEDIA_NS: u64 = 500_000_000;

/// Default fill watermark (bytes): 1 MiB buffered before `High`.
pub const DEFAULT_FILL_BYTES: u64 = 1024 * 1024;

/// Default drain watermark (bytes): 64 KiB buffered trips `Low`.
pub const DEFAULT_DRAIN_BYTES: u64 = 64 * 1024;

/// Default stall-prediction horizon: a predicted underrun inside
/// 2 s of wall time raises [`BufferSignal::StallPredicted`].
pub const DEFAULT_STALL_WINDOW_NS: u64 = 2_000_000_000;

/// The hysteresis watermark pair, per axis (module docs).
///
/// Invariants enforced by [`BufferWatermarks::validate`]:
/// `drain < fill` on both axes (equal thresholds would remove the
/// hysteresis band and flap at the boundary) and all values
/// positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferWatermarks {
    /// Buffered media duration at which `Low → High` fires
    /// (together with the byte axis).
    pub fill_media_ns: u64,
    /// Buffered media duration below which `High → Low` fires.
    pub drain_media_ns: u64,
    /// Buffered bytes at which `Low → High` fires (together with
    /// the duration axis).
    pub fill_bytes: u64,
    /// Buffered bytes below which `High → Low` fires.
    pub drain_bytes: u64,
}

impl Default for BufferWatermarks {
    fn default() -> Self {
        BufferWatermarks {
            fill_media_ns: DEFAULT_FILL_MEDIA_NS,
            drain_media_ns: DEFAULT_DRAIN_MEDIA_NS,
            fill_bytes: DEFAULT_FILL_BYTES,
            drain_bytes: DEFAULT_DRAIN_BYTES,
        }
    }
}

impl BufferWatermarks {
    /// Typed validation (never a clamp, MP §5): both axes strictly
    /// ordered `drain < fill`, every threshold positive.
    ///
    /// # Errors
    /// [`String`] describing the violated invariant (a configuration
    /// bug — callers surface it through their own typed config
    /// errors; the model itself keeps no I/O vocabulary).
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.fill_media_ns == 0
            || self.drain_media_ns == 0
            || self.fill_bytes == 0
            || self.drain_bytes == 0
        {
            return Err("watermark thresholds must be positive");
        }
        if self.drain_media_ns >= self.fill_media_ns {
            return Err("drain_media_ns must be below fill_media_ns (hysteresis band)");
        }
        if self.drain_bytes >= self.fill_bytes {
            return Err("drain_bytes must be below fill_bytes (hysteresis band)");
        }
        Ok(())
    }
}

/// The watermark state (hysteresis band between the thresholds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BufferLevel {
    /// Below the fill watermarks (or never filled): the resume gate
    /// is closed.
    #[default]
    Low,
    /// The fill watermarks were met; drains below the drain
    /// watermarks re-enter `Low`.
    High,
}

/// What one observation made the model do (all `Copy` — the
/// observation path allocates nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BufferSignal {
    /// No transition (the steady answer).
    #[default]
    Steady,
    /// `Low → High`: both fill watermarks met. Closes the open
    /// rebuffer (or initial-fill) episode.
    Filled,
    /// `High → Low`: a drain watermark tripped. Opens an episode.
    Drained,
    /// The stall predictor newly predicts an underrun inside the
    /// horizon (rising edge; carries the honest ETA).
    StallPredicted {
        /// Predicted wall-time until the runway is exhausted
        /// (nanoseconds) — an estimate from EWMA rates, never a
        /// promise.
        eta_ns: u64,
    },
    /// A previous prediction no longer holds (deficit closed or
    /// playback paused).
    StallCleared,
}

/// Rebuffer accounting (the metrics half of 6.D1): counts and
/// durations, initial fill separated from rebuffers (the first fill
/// is the stream's startup cost, not a network interruption).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RebufferMetrics {
    /// Media-time buffered at the last observation (both axes).
    pub buffered_media_ns: u64,
    /// Bytes buffered at the last observation.
    pub buffered_bytes: u64,
    /// Rebuffer episodes completed (drain → refill cycles after the
    /// initial fill).
    pub rebuffer_count: u64,
    /// Total wall time spent rebuffering across completed episodes.
    pub rebuffer_total_ns: u64,
    /// The longest completed rebuffer episode.
    pub rebuffer_longest_ns: u64,
    /// Wall time the current open episode has run (0 when none).
    pub current_episode_ns: u64,
    /// The initial fill's wall time (startup latency), once the
    /// first `Filled` lands.
    pub initial_fill_ns: Option<u64>,
}

/// The queryable state of the model (host polling surface; `Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferSnapshot {
    /// The watermark level.
    pub level: BufferLevel,
    /// Fill progress `0.0..=1.0` toward resuming: the **minimum** of
    /// the two axes' fractions (the bottleneck axis is the honest
    /// progress — the same shape the engine's per-limb fill takes).
    pub fill_fraction: f32,
    /// The metrics ledger.
    pub metrics: RebufferMetrics,
    /// EWMA consumption rate (media-ns consumed per wall-ms);
    /// `None` before two observations.
    pub consumption_rate: Option<u128>,
    /// EWMA arrival rate (media-ns received per wall-ms); `None`
    /// before two observations.
    pub arrival_rate: Option<u128>,
    /// The current stall forecast (`eta_ns` against the horizon).
    pub stall_eta_ns: Option<u64>,
}

/// One host observation of a network buffer (module docs for the
/// derivation of the arrival rate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferObservation {
    /// Caller-clock nanoseconds of this observation.
    pub now_ns: u64,
    /// Bytes buffered right now (the network side's own accounting:
    /// window-cache + retained bytes, whatever the source holds).
    pub buffered_bytes: u64,
    /// Media duration buffered right now (the runway truth).
    pub buffered_media_ns: u64,
    /// Media time consumed **since the previous observation**
    /// (playback progress delta; 0 while paused). A negative event
    /// (a seek re-anchor) is expressed by the host calling
    /// [`BufferModel::reset_rates`] instead of a wrapped negative.
    pub consumed_delta_ns: u64,
    /// Whether playback is actively consuming (paused playback
    /// drains nothing — no stall can be predicted while false).
    pub consuming: bool,
}

/// The adaptive buffer model (module docs). Construct with
/// [`BufferModel::new`], drive with
/// [`BufferModel::observe`](BufferModel::observe), query with
/// [`BufferModel::snapshot`](BufferModel::snapshot).
#[derive(Debug, Clone)]
pub struct BufferModel {
    watermarks: BufferWatermarks,
    /// Stall-prediction horizon (nanoseconds of wall time).
    stall_window_ns: u64,
    level: BufferLevel,
    metrics: RebufferMetrics,
    /// EWMA rates: media-ns per wall-ms.
    consumption_rate: Option<u128>,
    arrival_rate: Option<u128>,
    /// The previous observation (rate derivation).
    prev: Option<(u64 /* now_ns */, u64 /* buffered_media_ns */)>,
    /// The open episode's start (wall ns) — `None` when `High`.
    episode_started_ns: Option<u64>,
    /// Whether the initial fill has completed.
    initial_filled: bool,
    /// Whether a stall prediction is currently outstanding.
    stall_predicted: bool,
    /// The ETA the outstanding prediction carries (0 when none) —
    /// kept so [`BufferModel::snapshot`] can answer without
    /// recomputation (the query path stays allocation- and
    /// division-free).
    last_eta_ns: u64,
}

impl BufferModel {
    /// A model with the given watermarks and stall horizon. The
    /// initial-fill episode is open from construction (the first
    /// [`BufferSignal::Filled`] books the startup latency into
    /// [`RebufferMetrics::initial_fill_ns`]).
    ///
    /// # Errors
    /// [`BufferWatermarks::validate`]'s verdict, verbatim.
    pub fn new(
        watermarks: BufferWatermarks,
        stall_window_ns: u64,
    ) -> Result<BufferModel, &'static str> {
        watermarks.validate()?;
        Ok(BufferModel {
            watermarks,
            stall_window_ns,
            level: BufferLevel::Low,
            metrics: RebufferMetrics::default(),
            consumption_rate: None,
            arrival_rate: None,
            prev: None,
            // The initial episode: the stream starts buffering the
            // moment it is constructed (epoch 0 on the caller's
            // clock — the model never reads a clock itself).
            episode_started_ns: Some(0),
            initial_filled: false,
            stall_predicted: false,
            last_eta_ns: 0,
        })
    }

    /// The configured watermarks (host telemetry).
    pub fn watermarks(&self) -> BufferWatermarks {
        self.watermarks
    }

    /// Forget the rate history (a seek or source re-anchor made it
    /// meaningless — the honest reset, never a fabricated rate).
    pub fn reset_rates(&mut self) {
        self.consumption_rate = None;
        self.arrival_rate = None;
        self.prev = None;
        if self.stall_predicted {
            self.stall_predicted = false;
        }
    }

    /// Observe the buffer (module docs). Returns the transition
    /// signal — `Steady` when nothing changed. Allocation-free.
    pub fn observe(&mut self, obs: BufferObservation) -> BufferSignal {
        // ---- Ledger (before any transition) ----
        self.metrics.buffered_media_ns = obs.buffered_media_ns;
        self.metrics.buffered_bytes = obs.buffered_bytes;

        // ---- Rates (EWMA, one division per observation) ----
        let mut signal = BufferSignal::Steady;
        if let Some((prev_ns, prev_buffered)) = self.prev {
            let elapsed_ms = obs.now_ns.saturating_sub(prev_ns) / 1_000_000;
            if elapsed_ms > 0 {
                // Consumption: media-ns per wall-ms.
                let consumption = u128::from(obs.consumed_delta_ns) / u128::from(elapsed_ms);
                self.consumption_rate = Some(match self.consumption_rate {
                    // 3/4 old + 1/4 new — shift-only fixed point (D-2).
                    Some(prev) => prev - (prev >> 2) + (consumption >> 2),
                    None => consumption,
                });
                // Arrival: the buffered delta plus what playback
                // consumed out of it (clamped at 0 — eviction and
                // seeks read as arrival stalls, never as negative
                // bandwidth).
                let buffered_delta = obs
                    .buffered_media_ns
                    .saturating_add(obs.consumed_delta_ns)
                    .saturating_sub(prev_buffered);
                let arrival = u128::from(buffered_delta) / u128::from(elapsed_ms);
                self.arrival_rate = Some(match self.arrival_rate {
                    Some(prev) => prev - (prev >> 2) + (arrival >> 2),
                    None => arrival,
                });
            }
        }
        self.prev = Some((obs.now_ns, obs.buffered_media_ns));

        // ---- Stall prediction (rising/falling edge) ----
        let stall_eta = self.predict_stall_ns(&obs);
        match (self.stall_predicted, stall_eta) {
            (false, Some(eta_ns)) => {
                self.stall_predicted = true;
                self.last_eta_ns = eta_ns;
                signal = BufferSignal::StallPredicted { eta_ns };
            }
            (true, None) => {
                self.stall_predicted = false;
                self.last_eta_ns = 0;
                signal = BufferSignal::StallCleared;
            }
            (true, Some(eta_ns)) => {
                // Still predicted: refresh the queryable ETA.
                self.last_eta_ns = eta_ns;
            }
            (false, None) => {}
        }

        // ---- Watermark state machine (hysteresis) ----
        match self.level {
            BufferLevel::Low => {
                let filled = obs.buffered_media_ns >= self.watermarks.fill_media_ns
                    && obs.buffered_bytes >= self.watermarks.fill_bytes;
                if filled {
                    self.level = BufferLevel::High;
                    // Episode close: book the duration into the
                    // ledger (initial fill vs rebuffer).
                    if let Some(started) = self.episode_started_ns.take() {
                        let duration = obs.now_ns.saturating_sub(started);
                        if self.initial_filled {
                            self.metrics.rebuffer_count += 1;
                            self.metrics.rebuffer_total_ns += duration;
                            self.metrics.rebuffer_longest_ns =
                                self.metrics.rebuffer_longest_ns.max(duration);
                        } else {
                            self.initial_filled = true;
                            self.metrics.initial_fill_ns = Some(duration);
                        }
                    }
                    signal = BufferSignal::Filled;
                }
            }
            BufferLevel::High => {
                let drained = obs.buffered_media_ns < self.watermarks.drain_media_ns
                    || obs.buffered_bytes < self.watermarks.drain_bytes;
                if drained {
                    self.level = BufferLevel::Low;
                    self.episode_started_ns = Some(obs.now_ns);
                    signal = BufferSignal::Drained;
                }
            }
        }

        // ---- The open episode's running age ----
        if let Some(started) = self.episode_started_ns {
            self.metrics.current_episode_ns = obs.now_ns.saturating_sub(started);
        } else {
            self.metrics.current_episode_ns = 0;
        }
        signal
    }

    /// The current stall forecast, if any (prediction core: deficit
    /// rate vs runway, inside the horizon).
    fn predict_stall_ns(&self, obs: &BufferObservation) -> Option<u64> {
        if !obs.consuming {
            return None; // paused playback drains nothing
        }
        let consumption = self.consumption_rate?;
        let arrival = self.arrival_rate.unwrap_or(0);
        // A shrinking-arrival transient below the EWMA is exactly
        // what prediction is for: compare against the *current*
        // deficit the EWMAs agree on.
        let deficit = consumption.saturating_sub(arrival);
        if deficit == 0 {
            return None; // holding or filling
        }
        let runway_media_ns = u128::from(obs.buffered_media_ns);
        let eta_ns = (runway_media_ns / deficit) * NS_PER_MS as u128;
        (eta_ns <= u128::from(self.stall_window_ns)).then_some(eta_ns as u64)
    }

    /// The queryable state (host polling).
    pub fn snapshot(&self) -> BufferSnapshot {
        let fill_fraction = (obs_fraction(
            self.metrics.buffered_media_ns,
            self.watermarks.fill_media_ns,
        )
        .min(obs_fraction(
            self.metrics.buffered_bytes,
            self.watermarks.fill_bytes,
        )))
        .clamp(0.0, 1.0);
        BufferSnapshot {
            level: self.level,
            fill_fraction,
            metrics: self.metrics,
            consumption_rate: self.consumption_rate,
            arrival_rate: self.arrival_rate,
            stall_eta_ns: (self.stall_predicted).then_some(self.last_eta_ns),
        }
    }
}

fn obs_fraction(value: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    (value as f32 / total as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// Helper: a model with small round watermarks for readable
    /// arithmetic.
    fn model() -> BufferModel {
        BufferModel::new(
            BufferWatermarks {
                fill_media_ns: 10_000,
                drain_media_ns: 2_000,
                fill_bytes: 1_000,
                drain_bytes: 200,
            },
            5_000,
        )
        .must()
    }

    fn obs(now_ns: u64, bytes: u64, media_ns: u64, consumed: u64) -> BufferObservation {
        BufferObservation {
            now_ns,
            buffered_bytes: bytes,
            buffered_media_ns: media_ns,
            consumed_delta_ns: consumed,
            consuming: true,
        }
    }

    /// Watermark validation rejects degenerate bands, typed.
    #[test]
    fn watermark_validation_rejects_degenerate_bands() {
        BufferWatermarks::default().validate().must();
        let mut bad = BufferWatermarks::default();
        bad.drain_media_ns = bad.fill_media_ns;
        assert!(bad.validate().is_err(), "no hysteresis band (media)");
        let mut zero = BufferWatermarks::default();
        zero.fill_bytes = 0;
        assert!(zero.validate().is_err(), "zero threshold");
        let mut inverted = BufferWatermarks::default();
        inverted.drain_bytes = inverted.fill_bytes + 1;
        assert!(inverted.validate().is_err(), "inverted byte band");
    }

    /// The hysteresis: fill needs BOTH axes, drain trips on EITHER,
    /// and the band between thresholds is steady.
    #[test]
    fn watermarks_fill_on_both_drain_on_either() {
        let mut m = model();
        // Both axes at fill: Filled.
        assert_eq!(m.observe(obs(1_000, 1_000, 10_000, 0)), BufferSignal::Filled);
        assert_eq!(m.snapshot().level, BufferLevel::High);
        // Inside the band (between drain and fill on both axes):
        // steady — hysteresis holds High.
        assert_eq!(m.observe(obs(2_000, 500, 5_000, 0)), BufferSignal::Steady);
        assert_eq!(m.snapshot().level, BufferLevel::High);
        // Byte axis below drain (duration still fine): Drained.
        assert_eq!(m.observe(obs(3_000, 100, 9_000, 0)), BufferSignal::Drained);
        assert_eq!(m.snapshot().level, BufferLevel::Low);
        // Refill, then the duration axis below drain (bytes fine):
        // Drained — either axis trips.
        assert_eq!(m.observe(obs(4_000, 1_000, 10_000, 0)), BufferSignal::Filled);
        assert_eq!(m.observe(obs(5_000, 900, 1_999, 0)), BufferSignal::Drained);
        // Refill on both axes: Filled again (a rebuffer episode).
        assert_eq!(m.observe(obs(20_000, 1_000, 10_000, 0)), BufferSignal::Filled);
    }

    /// Fill requires BOTH axes: byte-rich but duration-poor stays
    /// Low (and vice versa).
    #[test]
    fn fill_requires_both_axes() {
        let mut m = model();
        assert_eq!(m.observe(obs(1_000, 5_000, 5_000, 0)), BufferSignal::Steady);
        assert_eq!(
            m.snapshot().level,
            BufferLevel::Low,
            "duration axis short: no fill"
        );
        assert_eq!(m.observe(obs(2_000, 10, 50_000, 0)), BufferSignal::Steady);
        assert_eq!(
            m.snapshot().level,
            BufferLevel::Low,
            "byte axis short: no fill"
        );
        assert_eq!(m.observe(obs(3_000, 1_000, 10_000, 0)), BufferSignal::Filled);
    }

    /// Episode accounting: initial fill separated from rebuffers;
    /// durations and longest tracked.
    #[test]
    fn rebuffer_metrics_account_episodes() {
        let mut m = model();
        // Initial fill: opens implicitly at construction (epoch 0).
        m.observe(obs(1_000_000, 1_000, 10_000, 0)); // Filled at t=1ms
        let snap = m.snapshot();
        assert_eq!(snap.metrics.initial_fill_ns, Some(1_000_000));
        assert_eq!(snap.metrics.rebuffer_count, 0);
        // A rebuffer: drain at t=5ms, refill at t=12ms.
        m.observe(obs(5_000_000, 100, 1_000, 0)); // Drained
        assert_eq!(m.snapshot().metrics.current_episode_ns, 0, "same instant");
        m.observe(obs(8_000_000, 100, 1_000, 0));
        assert_eq!(
            m.snapshot().metrics.current_episode_ns,
            3_000_000,
            "running age of the open episode"
        );
        m.observe(obs(12_000_000, 1_000, 10_000, 0)); // Filled
        let snap = m.snapshot();
        assert_eq!(snap.metrics.rebuffer_count, 1);
        assert_eq!(snap.metrics.rebuffer_total_ns, 7_000_000);
        assert_eq!(snap.metrics.rebuffer_longest_ns, 7_000_000);
        assert_eq!(snap.metrics.current_episode_ns, 0, "episode closed");
        // A second, shorter rebuffer: longest stays the first.
        m.observe(obs(20_000_000, 100, 1_000, 0));
        m.observe(obs(23_000_000, 1_000, 10_000, 0));
        let snap = m.snapshot();
        assert_eq!(snap.metrics.rebuffer_count, 2);
        assert_eq!(snap.metrics.rebuffer_total_ns, 10_000_000);
        assert_eq!(snap.metrics.rebuffer_longest_ns, 7_000_000);
    }

    /// Stall prediction: a healthy stream (arrival = consumption)
    /// never predicts; an outage (arrival 0 while consuming 1x)
    /// drives the arrival EWMA down until the runway over the
    /// deficit lands inside the horizon — the ETA is honest, and
    /// pausing clears the prediction.
    #[test]
    fn stall_prediction_deficit_vs_runway() {
        let mut m = BufferModel::new(
            BufferWatermarks {
                fill_media_ns: 1_000,
                drain_media_ns: 100,
                fill_bytes: 1_000,
                drain_bytes: 100,
            },
            5_000_000_000, // 5 s horizon
        )
        .must();
        // Healthy: 1x consumption, 1x arrival, 10 s runway.
        m.observe(obs(0, 1_000, 10_000_000_000, 0)); // Filled (initial)
        m.observe(obs(1_000_000_000, 1_000, 10_000_000_000, 1_000_000_000));
        assert_eq!(
            m.observe(obs(2_000_000_000, 1_000, 10_000_000_000, 1_000_000_000)),
            BufferSignal::Steady,
            "balanced rates: no deficit, no prediction"
        );
        // Outage: arrival 0 (the runway drops as it is consumed).
        // The arrival EWMA (3/4 + 1/4) decays 1x → .75 → .56 → .42 →
        // .32 → .24 → .18 → …; the prediction fires once
        // runway/deficit ≤ 5 s — on this trace at t = 8 s with a
        // ~4.9 s ETA (asserted by bound, not exact value).
        let mut predicted_at = None;
        for t in 3u64..=10 {
            let now = t * 1_000_000_000;
            let runway = (10 - t + 2) * 1_000_000_000; // 9 s at t=3 … 2 s at t=10
            match m.observe(obs(now, 1_000, runway, 1_000_000_000)) {
                BufferSignal::StallPredicted { eta_ns } => {
                    assert!(eta_ns > 0 && eta_ns <= 5_000_000_000, "eta {eta_ns} in (0, 5s]");
                    predicted_at = Some(t);
                    break;
                }
                BufferSignal::Steady => {}
                other => unreachable!("unexpected signal at t={t}: {other:?}"),
            }
        }
        assert_eq!(predicted_at, Some(8), "prediction fires inside the horizon");
        assert!(m.snapshot().stall_eta_ns.is_some());
        // Paused playback clears the prediction (nothing drains).
        let mut paused = obs(11_000_000_000, 1_000, 1_000_000_000, 0);
        paused.consuming = false;
        assert_eq!(m.observe(paused), BufferSignal::StallCleared);
        assert_eq!(m.snapshot().stall_eta_ns, None);
    }

    /// A filling stream (arrival above consumption) never predicts.
    #[test]
    fn no_prediction_when_filling() {
        let mut m = model();
        m.observe(obs(0, 100, 1_000, 0));
        // Arrival 2x consumption: buffered grows each step.
        m.observe(obs(1_000_000_000, 300, 3_000_000_000, 1_000_000_000));
        m.observe(obs(2_000_000_000, 500, 5_000_000_000, 1_000_000_000));
        assert_eq!(m.observe(obs(3_000_000_000, 700, 7_000_000_000, 1_000_000_000)), BufferSignal::Steady);
        assert_eq!(m.snapshot().stall_eta_ns, None);
    }

    /// The runway outlives the horizon: no prediction (an honest
    /// miss-avoidance — only *imminent* underruns fire).
    #[test]
    fn no_prediction_when_runway_outlives_horizon() {
        let mut m = BufferModel::new(
            BufferWatermarks {
                fill_media_ns: 10_000,
                drain_media_ns: 2_000,
                fill_bytes: 10_000,
                drain_bytes: 2_000,
            },
            2_000_000_000, // 2 s horizon
        )
        .must();
        m.observe(obs(0, 10_000, 100_000_000_000, 0));
        // Deficit: consumption 2x, arrival 1x → deficit 1x; runway
        // ~98 s → ETA 98 s ≫ 2 s horizon.
        m.observe(obs(1_000_000_000, 10_000, 99_000_000_000, 2_000_000_000));
        assert_eq!(
            m.observe(obs(2_000_000_000, 10_000, 98_000_000_000, 2_000_000_000)),
            BufferSignal::Steady
        );
        assert_eq!(m.snapshot().stall_eta_ns, None);
    }

    /// reset_rates forgets history (the seek re-anchor contract).
    #[test]
    fn reset_rates_forgets_history() {
        let mut m = model();
        m.observe(obs(0, 1_000, 4_000_000_000, 0));
        m.observe(obs(1_000_000_000, 1_000, 4_000_000_000, 8_000_000_000));
        assert!(m.snapshot().consumption_rate.is_some());
        m.reset_rates();
        let snap = m.snapshot();
        assert_eq!(snap.consumption_rate, None);
        assert_eq!(snap.arrival_rate, None);
        assert_eq!(snap.stall_eta_ns, None);
    }

    /// The fill fraction is the bottleneck axis, clamped to 1.0.
    #[test]
    fn fill_fraction_is_bottleneck_axis() {
        let mut m = model();
        m.observe(obs(0, 500, 10_000, 0));
        let snap = m.snapshot();
        // bytes 500/1000, media 10000/10000 → 0.5.
        assert!((snap.fill_fraction - 0.5).abs() < 1e-6);
        m.observe(obs(1, 2_000, 5_000, 0));
        let snap = m.snapshot();
        // bytes 2.0 (clamped 1.0), media 0.5 → 0.5.
        assert!((snap.fill_fraction - 0.5).abs() < 1e-6);
    }

    /// A synthetic 3G/lossy trace (the 6.D1 AC): good-bandwidth
    /// stretches with two deep dips, one of which drains the buffer
    /// (rebuffer) and one of which the predictor sees coming. The
    /// honest verdict is recorded in the card report, not asserted
    /// into success — here we assert the model *reacts* truthfully.
    #[test]
    fn synthetic_3g_trace_reacts_truthfully() {
        let mut m = BufferModel::new(
            BufferWatermarks {
                fill_media_ns: 3_000_000_000,   // 3 s fill
                drain_media_ns: 500_000_000,    // 500 ms drain
                fill_bytes: 64 * 1024,          // bytes axis loose
                drain_bytes: 16 * 1024,         // (3G audio-scale)
            },
            2_000_000_000,                     // 2 s horizon
        )
        .must();
        // Consumption: 1x realtime (media == wall). Arrival: 2x for
        // the fill stretch, then a 4 s outage (arrival 0) while
        // consuming, then recovery at 3x.
        let mut now_ns = 0u64;
        let mut buffered = 0u64;
        let mut signals = Vec::new();
        // Fill phase: 2x arrival for 3 s → the buffer reaches 3 s.
        for _ in 0..30 {
            now_ns += 100_000_000; // 100 ms steps
            let consumed = 100_000_000;
            buffered = buffered.saturating_sub(consumed) + 200_000_000;
            buffered = buffered.min(10_000_000_000);
            signals.push(m.observe(obs(now_ns, 64 * 1024, buffered, consumed)));
        }
        assert!(
            signals.contains(&BufferSignal::Filled),
            "the good stretch fills the buffer"
        );
        // The initial fill books 2.9 s (the 2x stream reaches 3 s of
        // runway at the 29th 100 ms step — the first step's
        // consumption runs against an empty buffer).
        assert_eq!(m.snapshot().metrics.initial_fill_ns, Some(2_900_000_000));
        // Outage: 4 s of zero arrival while consuming 1x. The
        // predictor fires well before the drain watermark trips (on
        // this trace ~1.4 s earlier — the EWMA smoothing constant
        // sets the lead time; the assert records the ordering, which
        // is the point of prediction).
        let mut drain_at = None;
        let mut predicted_at = None;
        for step in 1..=40u64 {
            now_ns += 100_000_000;
            let consumed = 100_000_000;
            buffered = buffered.saturating_sub(consumed);
            match m.observe(obs(now_ns, 32 * 1024, buffered, consumed)) {
                BufferSignal::Drained => drain_at = Some(step),
                BufferSignal::StallPredicted { .. } => predicted_at = Some(step),
                _ => {}
            }
        }
        assert_eq!(drain_at, Some(27), "the outage drains the buffer at 2.7 s in");
        assert_eq!(
            predicted_at,
            Some(13),
            "the predictor fires ~1.4 s before the drain on this trace"
        );
        let snap = m.snapshot();
        assert_eq!(snap.metrics.rebuffer_count, 0, "episode still open until refill");
        // Recovery: 3x arrival refills.
        for _ in 0..30 {
            now_ns += 100_000_000;
            let consumed = 100_000_000;
            buffered = buffered.saturating_sub(consumed) + 300_000_000;
            buffered = buffered.min(10_000_000_000);
            if m.observe(obs(now_ns, 64 * 1024, buffered, consumed)) == BufferSignal::Filled {
                break;
            }
        }
        let snap = m.snapshot();
        assert_eq!(snap.metrics.rebuffer_count, 1, "the episode closed on refill");
        // The episode ran from the drain (t = 5.7 s: 2.7 s into the
        // 4 s outage) to the recovery refill (t = 8.5 s).
        assert_eq!(snap.metrics.rebuffer_longest_ns, 2_800_000_000);
    }
}
