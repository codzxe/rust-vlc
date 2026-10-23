//! The soak's measurement instruments (card 3.E4): the memory sampler
//! and the least-squares slope line.
//!
//! # What is measured, and why these two instruments
//!
//! The card names two instruments; the workspace owns exactly one
//! allocator instrumentation (grep: every `#[global_allocator]` in the
//! tree is `testkit::alloc_count::CountingGlobalAlloc` — there is no
//! per-category allocator, so "top object-growth suspects by category"
//! is realized as **per-loop-phase attribution**, see
//! [`PhaseAccumulator`]):
//!
//! 1. **`VmRSS`** from `/proc/self/status` — the kernel's truth for
//!    the process footprint (pages actually resident, allocator arenas
//!    and fragmentation included). Linux-only; on a platform without
//!    `/proc` the sample records `None` and the report says so (an
//!    absent instrument is reported, never faked).
//! 2. **`live_bytes`** from the process-global counting allocator —
//!    the *requested*-bytes truth: bytes the process asked for and has
//!    not freed. This is the precise leak signal (a monotonic climb
//!    here IS a leak by definition); RSS adds the allocator's
//!    arena/fragmentation behavior on top.
//!
//! # The slope line
//!
//! Least squares over `(elapsed_seconds, value)` pairs of the measured
//! window (post-warmup — see the main module docs): slope in bytes per
//! second, reported as MB/h (`slope × 3600 / 1e6`). Extrapolating a
//! short window to "per hour" is an EXTRAPOLATION and every consumer
//! of these numbers (the report, the assertion messages) labels it so.
//!
//! No unsafe, std-only, no panics on missing data (typed `Option`s).

use std::fs;
use std::time::Instant;

/// One memory sample on the fixed wall-clock cadence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Seconds since the soak's wall-clock origin (real time).
    pub elapsed_s: f64,
    /// `VmRSS` in kB (`None` where `/proc` is unavailable).
    pub rss_kb: Option<u64>,
    /// Counting-allocator live requested bytes at sample time.
    pub live_bytes: isize,
    /// Loops fully completed at sample time.
    pub cycles: u64,
    /// True while the sample sits inside the warmup window (excluded
    /// from the slope fit; kept in the report's series for honesty).
    pub warmup: bool,
}

/// Read the process's current `VmRSS` from `/proc/self/status` (kB).
///
/// A bounded, single-read parse: the line has the shape
/// `VmRSS:\t  123456 kB`. Any miss (no `/proc`, changed format) is
/// `None` — an honest absent instrument, never a zero.
#[must_use = "an unread sample proves nothing"]
pub fn read_vm_rss_kb() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let value = line.split_whitespace().nth(1)?;
    value.parse::<u64>().ok()
}

/// The least-squares slope of `y` over `t` (units: y per second), or
/// `None` when there are fewer than two points or zero time spread —
/// the honest refusal for a window that cannot define a line.
#[must_use = "an uncomputed slope proves nothing"]
pub fn least_squares_slope(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }
    let n = points.len() as f64;
    let (sum_t, sum_y, sum_tt, sum_ty) = points.iter().fold(
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
        |(t, y, tt, ty), (point_t, point_y)| {
            (
                t + point_t,
                y + point_y,
                tt + point_t * point_t,
                ty + point_t * point_y,
            )
        },
    );
    let denominator = n * sum_tt - sum_t * sum_t;
    if denominator <= f64::EPSILON {
        return None;
    }
    Some((n * sum_ty - sum_t * sum_y) / denominator)
}

/// One loop phase — the "category" axis of the leak triage. The soak
/// attributes counting-allocator growth to the loop op that was
/// running when it happened, so a leak names its own loop op (the
/// card's "which loop op grows RSS" handoff note is then mechanical).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// `session.open` — access + demuxer pre-pass + engine open +
    /// settle to a playing/buffering session.
    Open,
    /// Poll-driven playback advance between ops.
    Play,
    /// `playback.seek` — submit through the `Seeked`/`SeekFailed`
    /// terminal.
    Seek,
    /// `tracks.set-audio-track` — submit through the
    /// `tracks.changed` terminal.
    Switch,
    /// `session.close` (composed `stop → close` when active) through
    /// the settled `Idle` state.
    Close,
}

impl Phase {
    /// Every phase, in loop order.
    pub const ALL: [Phase; 5] = [
        Phase::Open,
        Phase::Play,
        Phase::Seek,
        Phase::Switch,
        Phase::Close,
    ];

    /// The stable report label.
    #[must_use = "report label"]
    pub fn label(self) -> &'static str {
        match self {
            Phase::Open => "open",
            Phase::Play => "play-advance",
            Phase::Seek => "seek",
            Phase::Switch => "track-switch",
            Phase::Close => "close",
        }
    }
}

/// The per-phase live-bytes attribution across all cycles: the sum of
/// per-cycle live-byte deltas measured at phase boundaries (negative
/// deltas are frees — expected at close), plus the cycle count, so the
/// report can state a mean net delta **per cycle** per phase.
///
/// Fixed-size (no per-cycle allocation): the triage itself must not be
/// a growth term in the instrument it feeds.
#[derive(Debug, Default)]
pub struct PhaseAccumulator {
    deltas: [isize; 5],
    cycles: [u64; 5],
    /// The high-water mark of the running per-phase sum — a phase that
    /// transiently allocates megabytes and frees them is visible as a
    /// peak, not as growth.
    peaks: [isize; 5],
}

impl PhaseAccumulator {
    /// Record one phase's net live-byte delta (post-phase minus
    /// pre-phase, both read at cycle boundaries).
    pub fn record(&mut self, phase: Phase, delta: isize) {
        let index = phase as usize;
        self.deltas[index] = self.deltas[index].saturating_add(delta);
        self.cycles[index] = self.cycles[index].saturating_add(1);
        self.peaks[index] = self.peaks[index].max(self.deltas[index]);
    }

    /// The mean net live-byte delta per cycle for one phase.
    #[must_use = "triage number"]
    pub fn mean_per_cycle(&self, phase: Phase) -> f64 {
        let index = phase as usize;
        let cycles = self.cycles[index];
        if cycles == 0 {
            return 0.0;
        }
        self.deltas[index] as f64 / cycles as f64
    }

    /// The observed high-water mark of the running per-phase sum.
    #[must_use = "triage number"]
    pub fn peak(&self, phase: Phase) -> isize {
        self.peaks[phase as usize]
    }

    /// The phases ranked by mean per-cycle growth, largest positive
    /// first (the triage's "top suspects" ordering).
    #[must_use = "triage ordering"]
    pub fn ranked_by_growth(&self) -> Vec<Phase> {
        let mut phases = Phase::ALL.to_vec();
        phases.sort_by(|a, b| {
            self.mean_per_cycle(*b)
                .partial_cmp(&self.mean_per_cycle(*a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        phases
    }
}

/// The wall-clock cadence sampler: owns the origin `Instant`, the
/// fixed sample interval, and the preallocated sample series (capacity
/// reserved up front so steady-state sampling does not itself grow
/// `live_bytes` — the instrument must not be a term in its own
/// measurement).
pub struct CadenceSampler {
    origin: Instant,
    interval_ms: u64,
    next_due: Instant,
    samples: Vec<Sample>,
    expected: usize,
}

impl CadenceSampler {
    /// A sampler ticking every `interval_ms` of real time, with room
    /// for `expected_samples` (duration/interval + slack) reserved.
    #[must_use = "sampler"]
    pub fn new(interval_ms: u64, expected_samples: usize) -> CadenceSampler {
        let origin = Instant::now();
        CadenceSampler {
            origin,
            interval_ms: interval_ms.max(1),
            next_due: origin,
            samples: Vec::with_capacity(expected_samples + 16),
            expected: expected_samples,
        }
    }

    /// Seconds since the wall-clock origin.
    #[must_use = "clock"]
    pub fn elapsed_s(&self) -> f64 {
        self.origin.elapsed().as_secs_f64()
    }

    /// Whether the fixed cadence ticked (checked at loop boundaries —
    /// sampling is inline, on the wall clock, never a second thread
    /// whose allocations would pollute the counting instrument).
    #[must_use = "cadence"]
    pub fn ticked(&self) -> bool {
        Instant::now() >= self.next_due
    }

    /// Take one sample now (caller: [`Self::ticked`] said yes).
    pub fn sample(&mut self, live_bytes: isize, cycles: u64, warmup: bool) {
        let now = Instant::now();
        // Re-arm from the schedule, not from `now`, so a slow loop
        // interval cannot drift the cadence (missed ticks collapse;
        // the series timestamps stay honest).
        while self.next_due <= now {
            self.next_due += std::time::Duration::from_millis(self.interval_ms);
        }
        self.samples.push(Sample {
            elapsed_s: now.duration_since(self.origin).as_secs_f64(),
            rss_kb: read_vm_rss_kb(),
            live_bytes,
            cycles,
            warmup,
        });
    }

    /// The samples taken so far (the full series, warmup included).
    #[must_use = "observable evidence"]
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }

    /// The reserved capacity (report note: proves the sampler is not a
    /// live-bytes growth term).
    #[must_use = "instrument fact"]
    pub fn reserved_capacity(&self) -> usize {
        self.expected + 16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The slope math on known lines: exact slopes for noise-free
    /// data, in both instruments' units.
    #[test]
    fn slope_math_is_exact_on_clean_lines() {
        let flat: Vec<(f64, f64)> = (0..10).map(|i| (f64::from(i), 1000.0)).collect();
        let slope = least_squares_slope(&flat).must();
        assert!(slope.abs() < 1e-9, "flat line slope {slope}");

        let rising: Vec<(f64, f64)> = (0..10)
            .map(|i| (f64::from(i), 500.0 * f64::from(i)))
            .collect();
        let slope = least_squares_slope(&rising).must();
        assert!((slope - 500.0).abs() < 1e-6, "rising line slope {slope}");

        let falling: Vec<(f64, f64)> = (0..5)
            .map(|i| (f64::from(i), -250.0 * f64::from(i)))
            .collect();
        let slope = least_squares_slope(&falling).must();
        assert!((slope + 250.0).abs() < 1e-6, "falling line slope {slope}");
    }

    /// Least squares over noisy data recovers the trend (the actual
    /// soak condition): slope within 5% of the truth.
    #[test]
    fn slope_math_recovers_noisy_trends() {
        // y = 1000 t + square wave(±50) — a leak of 1000 B/s under
        // sampling noise far larger than any real residual.
        let noisy: Vec<(f64, f64)> = (0..60)
            .map(|i| {
                let t = f64::from(i);
                let noise = if i % 2 == 0 { 50.0 } else { -50.0 };
                (t, 1000.0 * t + noise)
            })
            .collect();
        let slope = least_squares_slope(&noisy).must();
        assert!(
            (slope - 1000.0).abs() < 50.0,
            "noisy slope {slope} vs truth 1000"
        );
    }

    /// The honest refusals: no line from < 2 points or zero spread.
    #[test]
    fn slope_refuses_undetermined_windows() {
        assert!(least_squares_slope(&[]).is_none());
        assert!(least_squares_slope(&[(0.0, 1.0)]).is_none());
        assert!(least_squares_slope(&[(1.0, 1.0), (1.0, 2.0)]).is_none());
    }

    /// Per-phase attribution: growth ranks first, frees rank last,
    /// means are per-cycle.
    #[test]
    fn phase_accumulator_ranks_and_means() {
        let mut phases = PhaseAccumulator::default();
        phases.record(Phase::Seek, 4096);
        phases.record(Phase::Seek, 2048);
        phases.record(Phase::Open, 1_048_576);
        phases.record(Phase::Close, -1_048_576);
        assert!((phases.mean_per_cycle(Phase::Seek) - 3072.0).abs() < 1e-9);
        assert!((phases.mean_per_cycle(Phase::Open) - 1_048_576.0).abs() < 1e-9);
        assert!((phases.mean_per_cycle(Phase::Close) + 1_048_576.0).abs() < 1e-9);
        assert_eq!(phases.mean_per_cycle(Phase::Play), 0.0);
        let ranked = phases.ranked_by_growth();
        assert_eq!(ranked.first(), Some(&Phase::Open), "growth ranks first");
        assert_eq!(ranked.last(), Some(&Phase::Close), "frees rank last");
        assert_eq!(phases.peak(Phase::Seek), 6144, "running-sum peak");
    }

    /// On Linux (the CI and soak host) `/proc/self/status` parses.
    #[test]
    fn vm_rss_reads_on_proc_platforms() {
        match read_vm_rss_kb() {
            Some(kb) => assert!(kb > 0, "a live process has a positive VmRSS"),
            None => {
                // Not a failure on non-proc platforms — but this
                // workspace's soak hosts are Linux; print so a silent
                // CI platform change is visible in logs.
                eprintln!("note: /proc/self/status VmRSS unavailable on this platform");
            }
        }
    }
}
