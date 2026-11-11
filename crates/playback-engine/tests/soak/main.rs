//! The soak harness (card 3.E4, doc 13 wave 3.E; design: this
//! directory). The 4 h loop of open → play → seek → track-switch →
//! reopen, driven **through the command layer** (ARCH-5 — see
//! `dispatch.rs` for the recorded lineage of the in-test
//! `lumen_soak_dispatch`), sampling memory at a fixed wall-clock
//! cadence and asserting a bounded-memory slope.
//!
//! # Configuration (the nightly 4 h run is a config, not a hardcoded
//! long test)
//!
//! | env | default | meaning |
//! |---|---|---|
//! | `SOAK_MINUTES` | `2` (the CI-short default) | soak duration in **real** minutes (f64 — fractional values legal for smoke runs) |
//! | `SOAK_SAMPLE_MS` | `1000` | the fixed memory-sampling cadence (ms) |
//! | `SOAK_LEAK_MB_PER_H` | `50` ([`CEILING_MB_PER_H`]) | the **asserted** live-bytes slope ceiling (MB/h) — the nightly tightens it to the 5 MB/h design target |
//! | `SOAK_REPORT` | (unset) | write the full markdown report to this path |
//!
//! The nightly tier invocation is therefore:
//! `SOAK_MINUTES=240 SOAK_LEAK_MB_PER_H=5 cargo test -p playback-engine --test soak --release`
//! — wired as the scheduled job `.github/workflows/soak-nightly.yml`
//! (issue #18; its 5 MB/h ceiling is `[PROPOSAL]` until the first
//! real 4 h measurement says otherwise).
//!
//! # The measurement discipline
//!
//! - **Warmup, then measure**: the first 15% of the window (min 1 s,
//!   max 50% of the run, and at least one completed cycle) is the
//!   pool-sizing/allocator-steady-state warmup — its samples are
//!   marked and EXCLUDED from the slope fit (the
//!   `tests/reopen_alloc.rs` precedent). The series keeps them.
//! - **Two instruments**: `live_bytes` (the counting allocator —
//!   requested bytes not freed; the precise leak signal) and `VmRSS`
//!   (`/proc/self/status`; kernel truth including arenas).
//! - **The assertion** (the card's instruction): the measured-window
//!   live-bytes slope must stay under a GENEROUS ceiling
//!   ([`CEILING_MB_PER_H`], 50 MB/h unless `SOAK_LEAK_MB_PER_H`
//!   tightens it) so CI catches gross leaks without flaking; the
//!   nightly's 5 MB/h figure is the design TARGET `[PROPOSAL]`,
//!   asserted there only after a real measurement blesses it.
//! - **Liveness is asserted** (D-6): cycles must complete, ops must
//!   actually run (skip rates bounded), frames must decode — a soak
//!   that skips its ops is a ritual, not a measurement.
//!
//! # Test-binary discipline
//!
//! The counting allocator is process-global: every test in this
//! binary takes `serialize_tests()` first (the `reopen_alloc.rs`
//! precedent), so no sibling test thread allocates inside a measured
//! window.

mod dispatch;
mod memory;
mod report;
mod scenario;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use testkit::Must;
use testkit::alloc_count::{CountingGlobalAlloc, serialize_tests};

use dispatch::SoakHost;
use memory::{CadenceSampler, PhaseAccumulator, least_squares_slope};
use report::SoakReport;
use scenario::{CycleCounts, fixture_rotation, run_cycle};

/// The process-global counting allocator — the live-bytes instrument
/// (ED-FIX-2; the workspace's only allocator instrumentation).
#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

/// The live-bytes reader handed to the host's sampler (a plain fn —
/// sampling allocates nothing beyond the reserved sample slot).
fn live_bytes() -> isize {
    ALLOC.state().snapshot().live_bytes
}

/// The generous default ceiling (MB/h): gross leaks fail, noise does
/// not. The nightly tightens it via `SOAK_LEAK_MB_PER_H` toward the
/// card's 5 MB/h target `[PROPOSAL until first real measurement]`.
const CEILING_MB_PER_H: f64 = 50.0;
/// The warmup fraction of the run.
const WARMUP_FRACTION: f64 = 0.15;
/// Minimum completed cycles for the run to mean anything.
const MIN_CYCLES: u64 = 3;
/// Maximum share of playable cycles whose seek/switch op was skipped
/// for state reasons (above this the scenario is a ritual — D-6).
const MAX_SKIP_RATE: f64 = 0.25;

// ---------------------------------------------------------------------------
// Configuration (env-parsed; parse errors fail loudly, never default)
// ---------------------------------------------------------------------------

/// Parse the soak duration (minutes). `None` (unset) is the CI-short
/// default; a set-but-invalid value is an error sentence (dirty-world
/// parsing — garbage never silently becomes 2 minutes).
fn parse_minutes(value: Option<&str>) -> Result<f64, String> {
    let Some(text) = value else {
        return Ok(2.0);
    };
    let minutes: f64 = text
        .trim()
        .parse()
        .map_err(|_| format!("SOAK_MINUTES is not a number: {text:?}"))?;
    if !(0.0..=10_080.0).contains(&minutes) {
        return Err(format!(
            "SOAK_MINUTES={minutes} is outside (0, 10080] (a week); refusing"
        ));
    }
    if minutes <= 0.0 {
        return Err(format!("SOAK_MINUTES={minutes} must be positive"));
    }
    Ok(minutes)
}

/// Parse the sampling cadence (ms).
fn parse_sample_ms(value: Option<&str>) -> Result<u64, String> {
    let Some(text) = value else {
        return Ok(1000);
    };
    let ms: u64 = text
        .trim()
        .parse()
        .map_err(|_| format!("SOAK_SAMPLE_MS is not a number: {text:?}"))?;
    if ms == 0 || ms > 600_000 {
        return Err(format!(
            "SOAK_SAMPLE_MS={ms} is outside (0, 600000]; refusing"
        ));
    }
    Ok(ms)
}

/// The configured soak minutes (env `SOAK_MINUTES`).
fn soak_minutes() -> f64 {
    parse_minutes(std::env::var("SOAK_MINUTES").ok().as_deref()).must()
}

/// Parse the asserted leak ceiling (MB/h). `None` (unset) is the
/// generous default; a set-but-invalid value refuses (the
/// `SOAK_MINUTES` discipline — garbage never silently becomes 50).
fn parse_leak_mb_per_h(value: Option<&str>) -> Result<f64, String> {
    let Some(text) = value else {
        return Ok(CEILING_MB_PER_H);
    };
    let mb: f64 = text
        .trim()
        .parse()
        .map_err(|_| format!("SOAK_LEAK_MB_PER_H is not a number: {text:?}"))?;
    if !(0.0..=100_000.0).contains(&mb) {
        return Err(format!(
            "SOAK_LEAK_MB_PER_H={mb} is outside (0, 100000]; refusing"
        ));
    }
    if mb <= 0.0 {
        return Err(format!("SOAK_LEAK_MB_PER_H={mb} must be positive"));
    }
    Ok(mb)
}

/// The configured sample cadence (env `SOAK_SAMPLE_MS`).
fn soak_sample_ms() -> u64 {
    parse_sample_ms(std::env::var("SOAK_SAMPLE_MS").ok().as_deref()).must()
}

/// The configured leak ceiling (env `SOAK_LEAK_MB_PER_H`).
fn leak_mb_per_h() -> f64 {
    parse_leak_mb_per_h(std::env::var("SOAK_LEAK_MB_PER_H").ok().as_deref()).must()
}

/// The optional report path (env `SOAK_REPORT`).
fn report_path() -> Option<PathBuf> {
    std::env::var("SOAK_REPORT")
        .ok()
        .filter(|text| !text.is_empty())
        .map(PathBuf::from)
}

// ---------------------------------------------------------------------------
// The soak
// ---------------------------------------------------------------------------

/// THE soak: loop open → play → seek → track-switch → reopen on the
/// engine through the command layer for the configured REAL wall
/// duration, sample memory at the fixed cadence, fit the
/// least-squares slope over the measured window, and assert the
/// generous ceiling.
#[test]
fn soak_loops_stay_memory_bounded() {
    let _serial = serialize_tests();
    let minutes = soak_minutes();
    let sample_ms = soak_sample_ms();
    let ceiling = leak_mb_per_h();
    let duration = Duration::from_secs_f64(minutes * 60.0);
    let warmup = Duration::from_secs_f64(
        (duration.as_secs_f64() * WARMUP_FRACTION)
            .min(duration.as_secs_f64() * 0.5)
            .max(1.0),
    );

    let plans = fixture_rotation().must();
    let mut host = SoakHost::new();
    let expected_samples = (duration.as_secs_f64() / (sample_ms as f64 / 1000.0)).ceil() as usize;
    host.install_sampling(CadenceSampler::new(sample_ms, expected_samples), live_bytes);

    let start = Instant::now();
    let deadline = start + duration;
    let warmup_deadline = start + warmup;
    let mut phases = PhaseAccumulator::default();
    let mut counts = CycleCounts::default();
    let mut cycle_index = 0u64;
    let mut warmup_ended: Option<(f64, u64)> = None;

    while Instant::now() < deadline {
        let plan = &plans[(cycle_index as usize) % plans.len()];
        run_cycle(
            &mut host,
            plan,
            cycle_index,
            &mut phases,
            &mut counts,
            live_bytes,
        )
        .unwrap_or_else(|error| unreachable!("soak cycle {cycle_index} failed: {error}"));
        cycle_index += 1;
        if warmup_ended.is_none() && Instant::now() >= warmup_deadline && counts.cycles >= 1 {
            host.end_warmup();
            warmup_ended = Some((host.sampling_elapsed_s().must(), counts.cycles));
        }
    }
    // A run too short to end the warmup inside the loop still gets a
    // measured window (the final endpoint sample below joins it).
    if warmup_ended.is_none() {
        host.end_warmup();
        warmup_ended = Some((host.sampling_elapsed_s().must(), counts.cycles));
    }
    host.force_sample();

    let sampling = host.take_sampling().must();
    let samples = sampling.sampler.samples().to_vec();
    let (warmup_elapsed, warmup_cycles) = warmup_ended.must();
    let measured: Vec<&memory::Sample> = samples.iter().filter(|sample| !sample.warmup).collect();
    let measured_cycles = counts.cycles.saturating_sub(warmup_cycles);
    let measured_window_s = (sampling.sampler.elapsed_s() - warmup_elapsed).max(0.0);

    // ---- liveness (D-6): the loop actually looped ----
    assert!(
        counts.cycles >= MIN_CYCLES,
        "soak liveness: only {} cycles completed in {minutes} min — the run proves nothing",
        counts.cycles
    );
    let playable_cycles = counts
        .cycles
        .saturating_sub(counts.error_opens)
        .saturating_sub(counts.error_closes);
    let seek_skip_rate = counts.seeks_skipped as f64 / playable_cycles.max(1) as f64;
    // The switch op is per HOP (a there-and-back round trip is two
    // opportunities): the rate's denominator is the hop count.
    let switch_hops = playable_cycles.saturating_mul(2);
    let switch_skip_rate = counts.switches_skipped as f64 / switch_hops.max(1) as f64;
    assert!(
        seek_skip_rate <= MAX_SKIP_RATE,
        "soak liveness: {}/{} playable cycles skipped the seek op ({:.0}% > {:.0}%) — \
         the scenario is not exercising the seek path",
        counts.seeks_skipped,
        playable_cycles,
        seek_skip_rate * 100.0,
        MAX_SKIP_RATE * 100.0
    );
    assert!(
        switch_skip_rate <= MAX_SKIP_RATE,
        "soak liveness: {}/{} switch hops skipped for state ({:.0}% > {:.0}%) — \
         the scenario is not exercising the switch path (the truncated \
         fixture's drain-to-Ended between hops is the expected residual)",
        counts.switches_skipped,
        switch_hops,
        switch_skip_rate * 100.0,
        MAX_SKIP_RATE * 100.0
    );
    assert!(
        counts.seeks_ok + counts.seeks_failed >= 1,
        "soak liveness: no seek ever completed"
    );
    assert!(
        counts.switches_ok >= 1,
        "soak liveness: no track switch ever completed"
    );

    // ---- the slope fits (measured window only) ----
    let live_points: Vec<(f64, f64)> = measured
        .iter()
        .map(|sample| (sample.elapsed_s, sample.live_bytes as f64))
        .collect();
    let rss_points: Vec<(f64, f64)> = measured
        .iter()
        .filter_map(|sample| sample.rss_kb.map(|kb| (sample.elapsed_s, kb as f64)))
        .collect();
    let live_slope = least_squares_slope(&live_points);
    let rss_slope = least_squares_slope(&rss_points);

    if minutes >= 0.5 {
        assert!(
            measured.len() >= 2,
            "the measured window holds {} samples — too few to fit a line at {minutes} min",
            measured.len()
        );
    }

    // ---- THE assertion: the generous ceiling on live bytes ----
    if let Some(slope) = live_slope {
        let mb_per_h = slope * 3600.0 / 1e6;
        assert!(
            mb_per_h < ceiling,
            "soak bounded-memory: measured-window live_bytes slope is {slope:.1} B/s = \
             {mb_per_h:.3} MB/h (EXTRAPOLATION from a {:.1} s window), above the configured \
             {ceiling:.0} MB/h ceiling (SOAK_LEAK_MB_PER_H) — a leak the soak must name (see the \
             triage: {})",
            measured_window_s,
            triage_summary(&phases)
        );
    }

    // ---- the report (always printed; written to SOAK_REPORT if set) ----
    let stats = host.stats();
    let report = SoakReport {
        minutes,
        sample_ms,
        profile: option_env!("PROFILE").unwrap_or("unknown").to_string(),
        fixtures: plans.iter().map(|plan| plan.label.to_string()).collect(),
        samples,
        live_slope_b_per_s: live_slope,
        rss_slope_kb_per_s: rss_slope,
        ceiling_mb_per_h: ceiling,
        counts,
        phases,
        frames_delivered: stats.frames_delivered,
        decoders_reused: stats.decoders_reused,
        decoders_built: stats.decoders_built,
        polls: host.polls,
        events_seen: host.events_seen,
        receipts_seen: host.receipts_seen,
        switch_terminals: host.switch_terminals,
        sink_bytes: counts.decoded_bytes,
        reserved_samples: sampling.sampler.reserved_capacity(),
        measured_window_s,
        measured_cycles,
    };
    let markdown = report.markdown();
    if let Some(path) = report_path() {
        std::fs::write(&path, markdown.as_bytes()).unwrap_or_else(|error| {
            unreachable!("SOAK_REPORT write to {}: {error}", path.display())
        });
    }
    println!("\n{markdown}");
}

/// The one-line triage for assertion messages (the top suspect).
fn triage_summary(phases: &PhaseAccumulator) -> String {
    let ranked = phases.ranked_by_growth();
    let Some(top) = ranked.first() else {
        return "no phases recorded".to_string();
    };
    format!(
        "top suspect `{}` at {:+.1} B/cycle mean",
        top.label(),
        phases.mean_per_cycle(*top)
    )
}

// ---------------------------------------------------------------------------
// Config parsing tests (pure — env is process-global and unsafe to
// mutate under the 2024 edition + the no-unsafe lint)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_default_and_valid_shapes() {
        let _serial = serialize_tests();
        assert_eq!(
            parse_minutes(None).must(),
            2.0,
            "unset = the CI-short default"
        );
        assert_eq!(parse_minutes(Some("2")).must(), 2.0);
        assert_eq!(parse_minutes(Some("240")).must(), 240.0, "the nightly tier");
        assert_eq!(
            parse_minutes(Some("0.05")).must(),
            0.05,
            "fractional smoke runs"
        );
        assert!(
            parse_minutes(Some("bogus")).is_err(),
            "garbage refuses, never defaults"
        );
        assert!(parse_minutes(Some("0")).is_err(), "zero refuses");
        assert!(parse_minutes(Some("-1")).is_err(), "negative refuses");
        assert!(
            parse_minutes(Some("20000")).is_err(),
            "beyond a week refuses"
        );
    }

    #[test]
    fn leak_mb_per_h_default_and_valid_shapes() {
        let _serial = serialize_tests();
        assert_eq!(
            parse_leak_mb_per_h(None).must(),
            50.0,
            "unset = the generous default"
        );
        assert_eq!(
            parse_leak_mb_per_h(Some("5")).must(),
            5.0,
            "the nightly target [PROPOSAL]"
        );
        assert!(
            parse_leak_mb_per_h(Some("bogus")).is_err(),
            "garbage refuses, never defaults"
        );
        assert!(parse_leak_mb_per_h(Some("0")).is_err(), "zero refuses");
        assert!(parse_leak_mb_per_h(Some("-1")).is_err(), "negative refuses");
    }

    #[test]
    fn sample_ms_default_and_valid_shapes() {
        let _serial = serialize_tests();
        assert_eq!(parse_sample_ms(None).must(), 1000);
        assert_eq!(parse_sample_ms(Some("250")).must(), 250);
        assert!(parse_sample_ms(Some("0")).is_err());
        assert!(parse_sample_ms(Some("x")).is_err());
        assert!(parse_sample_ms(Some("1000000")).is_err());
    }
}
