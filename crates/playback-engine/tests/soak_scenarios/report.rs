//! The scenario-suite report (card 8.C1, issue #54 / P8-3): the
//! markdown artifact the suite emits — one section per scenario with
//! every number the run actually measured, the configured budgets it
//! was checked against, and the violations if any. Every per-hour
//! figure derived from a shorter window is labeled an EXTRAPOLATION
//! (the 3.E4 report discipline); every absent instrument is labeled
//! not-measurable, never zero.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use testkit::Must;

use crate::manifest::Manifest;
use crate::runner::{ScenarioCounts, SlopeVerdict};

/// One scenario's measured facts (built by the runners; nothing here
/// is invented).
pub struct ScenarioResult {
    /// The scenario id (the manifest key).
    pub id: String,
    /// The kind label.
    pub kind_label: &'static str,
    /// The driver axis as configured ("duration 8.00 min" / "1500
    /// cycles").
    pub driver: String,
    /// The fixture manifest paths driven.
    pub fixtures: Vec<String>,
    /// Wall seconds the scenario actually ran.
    pub wall_s: f64,
    /// Total samples taken (warmup included).
    pub samples_total: usize,
    /// Samples inside the warmup window (excluded from fits).
    pub warmup_samples: usize,
    /// The measured window's span (s).
    pub measured_window_s: f64,
    /// Cycles completed (the whole scenario; the report states the
    /// warmup split where it matters).
    pub measured_cycles: u64,
    /// The completed-op counters.
    pub counts: ScenarioCounts,
    /// The measured-window live-bytes slope (B/s), if definable.
    pub live_slope_b_per_s: Option<f64>,
    /// The measured-window VmRSS slope (kB/s), if measurable.
    pub rss_slope_kb_per_s: Option<f64>,
    /// The fd-count baseline (count-driven scenarios; Linux only).
    pub fd_baseline: Option<u64>,
    /// The fd-count final sample.
    pub fd_final: Option<u64>,
    /// The fd-count maximum observed.
    pub fd_max: Option<u64>,
    /// The VmRSS baseline (kB).
    pub rss_baseline_kb: Option<u64>,
    /// The VmRSS final sample (kB).
    pub rss_final_kb: Option<u64>,
    /// The live-bytes baseline.
    pub live_baseline_bytes: Option<isize>,
    /// The live-bytes final sample.
    pub live_final_bytes: Option<isize>,
    /// The mean net live-bytes delta per measured cycle (the drift
    /// triage number; count-driven scenarios).
    pub live_mean_delta_per_cycle: Option<f64>,
    /// Whether the wall cap ended the run before the configured cycle
    /// count (reported, never hidden).
    pub wall_capped: bool,
    /// The configured cycle count (count-driven scenarios).
    pub cycles_configured: Option<u64>,
    /// The ceiling this run checked the slope against (MB/h).
    pub ceiling_mb_per_h: f64,
    /// The slope-budget verdict.
    pub verdict: SlopeVerdict,
    /// Budget/liveness violations (the test asserts this empty).
    pub failures: Vec<String>,
    /// Honest-absence notes (instruments not available on this
    /// platform, wall-cap shortfalls) — reported, never failed.
    pub notes: Vec<String>,
}

/// Render the full suite report.
#[must_use = "the artifact"]
pub fn suite_markdown(manifest: &Manifest, results: &[ScenarioResult]) -> String {
    let mut out = String::with_capacity(16 * 1024);
    let total_wall: f64 = results.iter().map(|result| result.wall_s).sum();

    let _ = writeln!(
        out,
        "# Soak scenario-suite report\n\n\
         - Manifest: `{}` ({} scenario(s))\n\
         - Profile: {} build, {}/{}\n\
         - Captured: {} (UTC)\n\
         - Total scenario wall: {:.1} min ({:.0} s)\n\
         - Method: every op is a catalog `WireRequest` through the soak dispatch seam onto the \
         real engine stack (access → AVI demux → MJPEG/PCM decode → counting sink) against the \
         committed `tests/fixtures/` media; fixtures' seek targets and track facts are read from \
         their recorded expectation TOMLs, never transcribed.\n\
         - Instruments: `live_bytes` (process-global counting allocator — the precise leak \
         signal), `VmRSS` (`/proc/self/status`), fd count (`/proc/self/fd`, count-driven \
         scenarios, Linux only).\n\
         - Warmup: the first {} of each scenario is excluded from the fits (pool sizing, \
         allocator steady state); the series keeps the samples.\n\
         - Slopes: least squares over the measured window. Every MB/h figure below is an \
         EXTRAPOLATION from that window, not a 4 h measurement.",
        manifest.source,
        manifest.scenarios.len(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        std::env::consts::OS,
        std::env::consts::ARCH,
        utc_now_iso8601(),
        total_wall / 60.0,
        total_wall,
        fraction_word(manifest.defaults.warmup_fraction),
    );

    // ---- the summary table ----
    let _ = writeln!(
        out,
        "\n## Summary\n\n\
         | scenario | kind | wall | cycles | live slope (MB/h) | RSS slope (MB/h) | verdict |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|");
    for result in results {
        let live = match result.verdict.live_mb_per_h {
            Some(mb) => format!("{mb:.3}"),
            None => "n/a".to_string(),
        };
        let rss = match result.rss_slope_kb_per_s {
            Some(slope) => format!("{:.3}", slope * 3600.0 / 1024.0),
            None => "n/a".to_string(),
        };
        let verdict = if result.failures.is_empty() {
            "PASS"
        } else {
            &format!("FAIL ({} violation(s))", result.failures.len())
        };
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} s | {} | {} | {} | {} |",
            result.id, result.kind_label, result.wall_s, result.measured_cycles, live, rss, verdict
        );
    }
    let worst = results
        .iter()
        .filter_map(|result| result.verdict.live_mb_per_h)
        .fold(f64::INFINITY, f64::max);
    if worst.is_finite() {
        let _ = writeln!(
            out,
            "\nWorst measured live-bytes slope across scenarios: **{worst:.3} MB/h** (an \
             EXTRAPOLATION from this run's windows; the configured design target is {} MB/h, \
             the asserted ceiling {} MB/h). The freeze-or-revise reading for the target lives \
             in `docs/soak/soak-report-*.md`, decided on the numbers above.",
            manifest.defaults.target_mb_per_h, manifest.defaults.leak_ceiling_mb_per_h
        );
    }

    // ---- per-scenario sections ----
    for result in results {
        write_scenario(&mut out, result, manifest);
    }

    // ---- the violations, collected ----
    let failures: Vec<&str> = results
        .iter()
        .flat_map(|result| result.failures.iter().map(String::as_str))
        .collect();
    if failures.is_empty() {
        let _ = writeln!(
            out,
            "\n## Verdict\n\nPASS — every scenario stayed inside its configured liveness and \
             memory budgets."
        );
    } else {
        let _ = writeln!(
            out,
            "\n## Verdict\n\nFAIL — {} violation(s):\n",
            failures.len()
        );
        for failure in failures {
            let _ = writeln!(out, "- {failure}");
        }
    }
    out
}

/// One scenario's detail section.
fn write_scenario(out: &mut String, result: &ScenarioResult, manifest: &Manifest) {
    let _ = writeln!(
        out,
        "\n## Scenario `{}` ({})\n\n\
         - Driver: {} (configured); wall {:.1} s.\n\
         - Fixtures: {}.\n\
         - Samples: {} total, {} warmup-excluded, measured window {:.1} s.\n\
         - Ceiling checked: {} MB/h; design target {} MB/h ({}).",
        result.id,
        result.kind_label,
        result.driver,
        result.wall_s,
        result.fixtures.join(", "),
        result.samples_total,
        result.warmup_samples,
        result.measured_window_s,
        result.ceiling_mb_per_h,
        manifest.defaults.target_mb_per_h,
        if manifest.defaults.assert_target {
            "ASSERTED by this profile"
        } else {
            "reported, not asserted (see docs/soak/ for the freeze-or-revise decision)"
        },
    );
    if result.wall_capped {
        let _ = writeln!(
            out,
            "- NOTE: the wall cap ended this scenario at {}/{} configured cycles (honest \
             shortfall, not a failure; recalibrate the manifest if unintended).",
            result.measured_cycles,
            result.cycles_configured.unwrap_or(0)
        );
    }

    let counts = result.counts;
    let _ = writeln!(
        out,
        "- Ops: {} cycles, {} opens ({} error-lane), {} plays, {} seeks ok / {} failed / {} \
         skipped, {} closes ({} error-lane), {} playlist passes.",
        counts.cycles,
        counts.opens_ok,
        counts.error_opens,
        counts.plays_ok,
        counts.seeks_ok,
        counts.seeks_failed,
        counts.seeks_skipped,
        counts.closes_ok,
        counts.error_closes,
        counts.playlist_passes
    );
    let _ = writeln!(
        out,
        "- Decoding: {} frames, {} payload bytes at the sinks (the D-6 liveness axis).",
        counts.frames_decoded, counts.decoded_bytes
    );

    match result.verdict.live_mb_per_h {
        Some(mb) => {
            let _ = writeln!(
                out,
                "- Live-bytes slope: {:.1} B/s = **{:.3} MB/h** (EXTRAPOLATION from a {:.1} s \
                 window) — {}",
                result.live_slope_b_per_s.unwrap_or(0.0),
                mb,
                result.measured_window_s,
                slope_words(mb, result.ceiling_mb_per_h, manifest)
            );
        }
        None => {
            let _ = writeln!(
                out,
                "- Live-bytes slope: not slope-fitted (this scenario asserts growth budgets, \
                 below)"
            );
        }
    }
    match result.rss_slope_kb_per_s {
        Some(slope) => {
            let _ = writeln!(
                out,
                "- VmRSS slope: {:.1} kB/s = {:.3} MB/h (EXTRAPOLATION; kernel truth, arenas \
                 included).",
                slope,
                slope * 3600.0 / 1024.0
            );
        }
        None => {
            let _ = writeln!(
                out,
                "- VmRSS slope: n/a (no measurable samples on this platform or window)."
            );
        }
    }

    // The count-driven growth numbers.
    if result.fd_baseline.is_some() || result.fd_final.is_some() {
        let _ = writeln!(
            out,
            "- fd count: baseline {} → final {} (max {}); budget {}.",
            result
                .fd_baseline
                .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
            result
                .fd_final
                .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
            result
                .fd_max
                .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
            manifest.defaults.fd_growth_budget
        );
    }
    if result.rss_baseline_kb.is_some() || result.rss_final_kb.is_some() {
        let _ = writeln!(
            out,
            "- VmRSS growth: {} kB → {} kB; budget {} MB.",
            result
                .rss_baseline_kb
                .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
            result
                .rss_final_kb
                .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
            manifest.defaults.rss_growth_budget_mb
        );
    }
    if result.live_baseline_bytes.is_some() || result.live_final_bytes.is_some() {
        let _ = writeln!(
            out,
            "- Live bytes: {} → {} (budget {} B).",
            result
                .live_baseline_bytes
                .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
            result
                .live_final_bytes
                .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
            manifest.defaults.live_growth_budget_bytes
        );
    }
    if let Some(delta) = result.live_mean_delta_per_cycle {
        let _ = writeln!(
            out,
            "- Mean net live-bytes drift: {delta:+.1} B/cycle over the measured cycles (the \
             reopen-stress triage number; negative is memory returning)."
        );
    }
    for note in &result.notes {
        let _ = writeln!(out, "- NOTE: {note}");
    }
    if !result.failures.is_empty() {
        let _ = writeln!(out, "- VIOLATIONS:");
        for failure in &result.failures {
            let _ = writeln!(out, "  - {failure}");
        }
    }
}

/// The verdict words for a measured slope against its budgets.
fn slope_words(mb_per_h: f64, ceiling: f64, manifest: &Manifest) -> String {
    if mb_per_h < 0.0 {
        "negative (memory returning)".to_string()
    } else if mb_per_h < manifest.defaults.target_mb_per_h {
        format!(
            "within the {} MB/h design target",
            manifest.defaults.target_mb_per_h
        )
    } else if mb_per_h < ceiling {
        format!(
            "ABOVE the {} MB/h design target, below the {} MB/h ceiling — investigate",
            manifest.defaults.target_mb_per_h, ceiling
        )
    } else {
        format!("AT/ABOVE the {ceiling} MB/h ceiling — a leak the soak must name")
    }
}

/// The warmup share in human words ("15%").
fn fraction_word(fraction: f64) -> String {
    format!("{:.0}%", fraction * 100.0)
}

// ---------------------------------------------------------------------------
// The UTC clock (std-only; the civil-from-days algorithm)
// ---------------------------------------------------------------------------

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// std-only (`SystemTime` has no calendar); the days→(y, m, d) mapping
/// is Howard Hinnant's `civil_from_days` (public-domain algorithm,
/// reproduced from the mathematical definition of the proleptic
/// Gregorian calendar — no third-party code).
#[must_use = "report timestamp"]
pub fn utc_now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    let rem = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole-binary serializer (the counting allocator is
    /// process-global; a sibling test's allocations inside a measured
    /// window are pollution).
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        testkit::alloc_count::serialize_tests()
    }

    /// Known dates (the algorithm is only as good as its checks):
    /// the epoch, a recent Jan 1, and a date in the report-writing
    /// week (verified against an independent calendar, not assumed).
    #[test]
    fn civil_dates_match_known_days() {
        let _serial = serial();
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1), "2024-01-01");
        assert_eq!(civil_from_days(20_768), (2026, 11, 11), "2026-11-11");
        assert_eq!(civil_from_days(-1), (1969, 12, 31), "pre-epoch");
    }

    /// The timestamp is shaped `YYYY-MM-DDTHH:MM:SSZ`.
    #[test]
    fn utc_timestamp_has_the_documented_shape() {
        let _serial = serial();
        let text = utc_now_iso8601();
        let bytes = text.as_bytes();
        assert_eq!(bytes.len(), 20, "{text}");
        assert_eq!(&text[4..5], "-");
        assert_eq!(&text[10..11], "T");
        assert!(text.ends_with('Z'), "{text}");
        let year: i64 = text[0..4].parse().must();
        assert!(
            (2020..=2100).contains(&year),
            "a plausible year, not garbage: {text}"
        );
    }
}
