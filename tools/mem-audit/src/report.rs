//! Report formatting + per-class aggregation (card W5-15).
//!
//! One machine-parseable `MEMAUDIT` line per file per repeat (the
//! audit's contract row), one `MEMSUM` line per class, one `MEMNOOP`
//! floor line per run. Everything is formatted OUTSIDE the
//! measurement windows (`walk.rs` discipline), so printing never
//! pollutes a window.
//!
//! # The MEMAUDIT line
//!
//! ```text
//! MEMAUDIT file=<path> class=<c> rep=<i> verdict=<decoded|refused|fatal> size=<B> container=<c> tracks=<n> limbs=<v+a|v|a|none> v_pkts=<n> a_pkts=<n> v_dec=<n> a_dec=<n> v_conc=<n> a_conc=<n> dmx_rec=<n> open_alloc_B=<n> open_calls=<n> open_peak_B=<n> open_base_B=<n> walk_alloc_B=<n> walk_calls=<n> walk_peak_B=<n> walk_base_B=<n> peak_B=<n> B_per_pkt=<f> calls_per_pkt=<f> [stage=<s>] [error=<verbatim to end of line>]
//! ```
//!
//! - `peak_B` = max(open peak, walk peak) — the union property of
//!   adjacent windows (a max over concatenated intervals is the max
//!   of the per-interval maxima), so the whole-run peak needs no
//!   third window.
//! - `B_per_pkt` / `calls_per_pkt` = walk-window allocation bytes /
//!   calls divided by packets FED to decoders (the D-1 detector:
//!   anything above zero in the steady state is a per-frame
//!   allocation site). `-` when no packet was fed.
//! - `error=` is always the LAST field (it may contain spaces).

use crate::walk::WalkReport;

/// One classified run request (class label + path, repeat handled by
/// the caller).
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The class label (explicit `CLASS=path` or the classifier).
    pub class: String,
    /// The path verbatim.
    pub path: String,
}

/// The auto-classifier (documented rules; an explicit `CLASS=path`
/// always wins). Order matters — first match answers.
///
/// | rule (on the lowercased path) | class |
/// |---|---|
/// | contains `/malformed/` or `__` | `fixture-malformed` |
/// | contains `/valid/` | `fixture-valid` |
/// | contains `heavy` | `aux-heavy` |
/// | contains `uhd4k` or `bigres` | `aux-bigres` |
/// | contains `audioonly` | `aux-audio-only` |
/// | contains `videoonly` | `aux-video-only` |
/// | contains `longlarge` | `aux-long-large` |
/// | anything else | `other` |
#[must_use]
pub fn classify(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.contains("/malformed/") || lower.contains("__") {
        "fixture-malformed"
    } else if lower.contains("/valid/") {
        "fixture-valid"
    } else if lower.contains("heavy") {
        "aux-heavy"
    } else if lower.contains("uhd4k") || lower.contains("bigres") {
        "aux-bigres"
    } else if lower.contains("audioonly") {
        "aux-audio-only"
    } else if lower.contains("videoonly") {
        "aux-video-only"
    } else if lower.contains("longlarge") {
        "aux-long-large"
    } else {
        "other"
    }
}

/// Parse one positional argument: `CLASS=path` (class is
/// `[a-z0-9_-]+`) or a bare path (classified).
#[must_use]
pub fn parse_entry(arg: &str) -> FileEntry {
    if let Some((class, path)) = arg.split_once('=')
        && !class.is_empty()
        && !path.is_empty()
        && class
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
    {
        return FileEntry {
            class: class.to_string(),
            path: path.to_string(),
        };
    }
    FileEntry {
        class: classify(arg).to_string(),
        path: arg.to_string(),
    }
}

/// The per-file result line (the exact contract — see the module
/// docs; `error=` last, verbatim).
#[must_use]
pub fn memaudit_line(report: &WalkReport, class: &str, rep: u32, path: &str) -> String {
    let fed = report.fed_packets();
    let per_pkt = |value: u64| -> String {
        if fed == 0 {
            "-".to_string()
        } else {
            format!("{:.2}", value as f64 / fed as f64)
        }
    };
    let peak = report.open.peak_live.max(report.walk.peak_live);
    let mut line = format!(
        "MEMAUDIT file={path} class={class} rep={rep} verdict={} size={} container={} \
tracks={} limbs={} v_pkts={} a_pkts={} v_dec={} a_dec={} v_conc={} a_conc={} dmx_rec={} \
open_alloc_B={} open_calls={} open_peak_B={} open_base_B={} \
walk_alloc_B={} walk_calls={} walk_peak_B={} walk_base_B={} peak_B={} B_per_pkt={} calls_per_pkt={}",
        verdict_name(report.verdict),
        report.size,
        report.container,
        report.tracks,
        report.limbs,
        report.v_pkts,
        report.a_pkts,
        report.v_dec,
        report.a_dec,
        report.v_concealed,
        report.a_concealed,
        report.demux_recovered,
        report.open.alloc_bytes,
        report.open.alloc_calls,
        report.open.peak_live,
        report.open.baseline_live,
        report.walk.alloc_bytes,
        report.walk.alloc_calls,
        report.walk.peak_live,
        report.walk.baseline_live,
        peak,
        per_pkt(report.walk.alloc_bytes),
        per_pkt(report.walk.alloc_calls),
    );
    if let Some(stage) = &report.stage {
        line.push_str(&format!(" stage={stage}"));
    }
    if let Some(error) = &report.error {
        line.push_str(&format!(" error={error}"));
    }
    line
}

/// The verdict's line name.
fn verdict_name(verdict: crate::walk::Verdict) -> &'static str {
    match verdict {
        crate::walk::Verdict::Decoded => "decoded",
        crate::walk::Verdict::Refused => "refused",
        crate::walk::Verdict::Fatal => "fatal",
    }
}

/// The no-op floor line (the instrument + process cost before any
/// audit work — see `selfcheck.rs`).
#[must_use]
pub fn memnoop_line(alloc_bytes: u64, alloc_calls: u64, live: u64) -> String {
    format!("MEMNOOP alloc_bytes={alloc_bytes} alloc_calls={alloc_calls} live={live}")
}

/// One class's aggregate over its runs.
#[derive(Debug, Clone)]
pub struct ClassSummary {
    /// The class label.
    pub class: String,
    /// Distinct files (refused files included — they are real
    /// outcomes, never silently dropped).
    pub files: usize,
    /// Total runs (repeats included).
    pub runs: usize,
    /// Runs that reached EndOfStream.
    pub decoded: usize,
    /// Open-phase refusals.
    pub refused: usize,
    /// Mid-walk fatals.
    pub fatal: usize,
    /// Median walk alloc calls per fed packet (decoded runs only;
    /// even counts average the two middles).
    pub med_calls_per_pkt: Option<f64>,
    /// Max ditto.
    pub max_calls_per_pkt: Option<f64>,
    /// Median walk alloc bytes per fed packet (decoded runs only).
    pub med_b_per_pkt: Option<f64>,
    /// Max ditto.
    pub max_b_per_pkt: Option<f64>,
    /// Max whole-run peak (all runs — refusals have peaks too).
    pub max_peak_b: u64,
    /// Max walk growth over baseline (the streaming-footprint
    /// column).
    pub max_growth_b: u64,
    /// Max file size in the class.
    pub max_size_b: u64,
}

/// The `MEMSUM` line for one class.
#[must_use]
pub fn memsum_line(summary: &ClassSummary) -> String {
    let opt = |value: &Option<f64>| -> String {
        match value {
            Some(value) => format!("{value:.2}"),
            None => "-".to_string(),
        }
    };
    format!(
        "MEMSUM class={} files={} runs={} decoded={} refused={} fatal={} \
med_calls_per_pkt={} max_calls_per_pkt={} med_B_per_pkt={} max_B_per_pkt={} \
max_peak_B={} max_growth_B={} max_size_B={}",
        summary.class,
        summary.files,
        summary.runs,
        summary.decoded,
        summary.refused,
        summary.fatal,
        opt(&summary.med_calls_per_pkt),
        opt(&summary.max_calls_per_pkt),
        opt(&summary.med_b_per_pkt),
        opt(&summary.max_b_per_pkt),
        summary.max_peak_b,
        summary.max_growth_b,
        summary.max_size_b,
    )
}

/// Median of a non-empty slice (even lengths average the two
/// middles — documented, standard).
fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    })
}

/// Aggregate one class's runs into a summary (reports + class label
/// + the DISTINCT file count — repeats must not inflate `files`).
#[must_use]
pub fn summarize(class: &str, reports: &[WalkReport], distinct_files: usize) -> ClassSummary {
    let mut calls: Vec<f64> = Vec::new();
    let mut bytes: Vec<f64> = Vec::new();
    let mut decoded = 0;
    let mut refused = 0;
    let mut fatal = 0;
    let mut max_peak = 0;
    let mut max_growth = 0;
    let mut max_size = 0;
    for report in reports {
        match report.verdict {
            crate::walk::Verdict::Decoded => {
                decoded += 1;
                let fed = report.fed_packets();
                if fed > 0 {
                    calls.push(report.walk.alloc_calls as f64 / fed as f64);
                    bytes.push(report.walk.alloc_bytes as f64 / fed as f64);
                }
            }
            crate::walk::Verdict::Refused => refused += 1,
            crate::walk::Verdict::Fatal => fatal += 1,
        }
        max_peak = max_peak
            .max(report.open.peak_live)
            .max(report.walk.peak_live);
        max_growth = max_growth
            .max(report.open.growth())
            .max(report.walk.growth());
        max_size = max_size.max(report.size);
    }
    let max_of = |values: &[f64]| {
        values.iter().copied().fold(None::<f64>, |acc, v| {
            Some(match acc {
                Some(acc) => acc.max(v),
                None => v,
            })
        })
    };
    ClassSummary {
        class: class.to_string(),
        files: distinct_files,
        runs: reports.len(),
        decoded,
        refused,
        fatal,
        med_calls_per_pkt: median(&mut calls),
        max_calls_per_pkt: max_of(&calls),
        med_b_per_pkt: median(&mut bytes),
        max_b_per_pkt: max_of(&bytes),
        max_peak_b: max_peak,
        max_growth_b: max_growth,
        max_size_b: max_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The classifier's documented rules.
    #[test]
    fn classifier_rules() {
        let _serial = crate::testutil::serial();
        assert_eq!(
            classify("tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc0.avi"),
            "fixture-malformed"
        );
        assert_eq!(
            classify("tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"),
            "fixture-valid"
        );
        assert_eq!(classify("media/heavy_720p_q7.avi"), "aux-heavy");
        assert_eq!(classify("media/uhd4k_q7.avi"), "aux-bigres");
        assert_eq!(classify("media/audioonly_s16.avi"), "aux-audio-only");
        assert_eq!(classify("media/videoonly_mjpeg.avi"), "aux-video-only");
        assert_eq!(classify("media/longlarge_stream.avi"), "aux-long-large");
        assert_eq!(classify("some/random/file.bin"), "other");
    }

    /// `CLASS=path` parsing (and that paths with `=` but invalid
    /// class characters fall back to classification).
    #[test]
    fn entry_parsing() {
        let _serial = crate::testutil::serial();
        let entry = parse_entry("aux-heavy=/tmp/x=1.avi");
        assert_eq!(entry.class, "aux-heavy");
        assert_eq!(entry.path, "/tmp/x=1.avi");
        let entry = parse_entry("NotAClass=/tmp/a.avi");
        assert_eq!(entry.class, "other");
        assert_eq!(entry.path, "NotAClass=/tmp/a.avi");
    }

    /// Median: odd takes the middle, even averages the two middles.
    #[test]
    fn median_shape() {
        let _serial = crate::testutil::serial();
        let mut odd = [3.0, 1.0, 2.0];
        assert_eq!(median(&mut odd), Some(2.0));
        let mut even = [4.0, 1.0, 3.0, 2.0];
        assert_eq!(median(&mut even), Some(2.5));
        let mut empty: [f64; 0] = [];
        assert_eq!(median(&mut empty), None);
    }
}
