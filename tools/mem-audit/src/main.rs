//! `lumen-mem-audit` — the memory measurement instrument (card
//! W5-15): a counting global allocator over the real first-vertical
//! decode path.
//!
//! ```text
//! Usage: lumen-mem-audit [--repeat N] [--self-check] [(CLASS=)FILE]...
//! ```
//!
//! - Each `FILE` is walked open → demux → decode-to-null with the
//!   counting allocator's windows bracketing the open and walk
//!   phases; one `MEMAUDIT` line is printed per file per repeat
//!   (the D-1 detector: `calls_per_pkt` above zero in the steady
//!   state is a per-frame allocation site). Since card 5.X2a the
//!   walk drives every landed demux family (AVI full decode; mkv/
//!   mp4/ogg demux drilldowns; wav + PCM decode) and every line
//!   carries the per-file peak-RSS pair `rss_hwm_kB`/`rss_base_kB`
//!   (`/proc/self/status` VmHWM/VmRSS, watermark reset via
//!   `/proc/self/clear_refs`).
//! - `CLASS=FILE` overrides the path classifier (class labels are
//!   `[a-z0-9_-]+`; see `report::classify` for the auto rules).
//! - `--repeat N` walks every file N times consecutively (the
//!   bounded-memory dimension-stability proof: run 2 ≈ run 1).
//! - `--self-check` measures the instrument itself (the no-op floor
//!   + the per-call counter cost) and exits.
//! - A run with no files prints the `MEMNOOP` floor line only — that
//!   IS the no-op run.
//!
//! Exit codes: 0 the requested runs completed (refusals and fatals
//! are honest per-file outcomes, not tool failures); 64 usage.
//!
//! # Unsafe posture
//!
//! This tool carries the minimal unsafe blocks a counting GLOBAL
//! allocator requires (`unsafe impl GlobalAlloc` +
//! `#[global_allocator]` in `allocator.rs`; the micro-bench's raw
//! allocator calls in `selfcheck.rs`) — there is no safe Rust
//! surface for either. It is a measurement instrument, not an
//! engine/codec crate; every block is safety-commented and the
//! posture is recorded for Lead ratification (Cargo.toml header).

mod allocator;
mod report;
mod selfcheck;
mod walk;

#[cfg(test)]
mod testutil {
    use std::sync::{Mutex, MutexGuard};

    /// This binary's tests share ONE global allocator, and cargo's
    /// default test runner is multi-threaded: without serialization,
    /// a concurrently-running test's allocations pollute every other
    /// test's windows (the instrument measures the whole process —
    /// that is its job). Every test in this crate takes this lock for
    /// its whole body; the counting-allocator unit tests need exact
    /// deltas, and the walk tests need clean windows.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// Acquire the whole-binary test lock (poison-recovered: a
    /// failed test's leftovers never block the diagnostic that
    /// follows).
    pub fn serial() -> MutexGuard<'static, ()> {
        SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

use std::io::Write;
use std::path::Path;

use allocator::{Counting, counting_global};

/// The process-wide counting allocator (card W5-15's instrument).
#[global_allocator]
static GLOBAL: Counting = counting_global();

/// Usage (stderr; exit 64 path).
const USAGE: &str =
    "Usage: lumen-mem-audit [--repeat N] [--self-check] [--demux-only] [(CLASS=)FILE]...";

/// The entry point: parse, dispatch, print. Never panics on bad
/// input — usage errors are typed lines + exit 64.
fn main() {
    let code = run(std::env::args().skip(1).collect());
    std::process::exit(i32::from(code));
}

/// The testable body of `main`.
fn run(args: Vec<String>) -> u8 {
    let mut repeat: u32 = 1;
    let mut self_check = false;
    let mut demux_only = false;
    let mut entries: Vec<report::FileEntry> = Vec::new();
    let mut args = args.into_iter().peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--self-check" => self_check = true,
            "--demux-only" => demux_only = true,
            "--repeat" => {
                let Some(value) = args.next() else {
                    eprintln!("error: --repeat needs a value; {USAGE}");
                    return 64;
                };
                match value.parse::<u32>() {
                    Ok(n) if (1..=1000).contains(&n) => repeat = n,
                    _ => {
                        eprintln!("error: --repeat needs an integer in 1..=1000; {USAGE}");
                        return 64;
                    }
                }
            }
            other if other.starts_with("--") => {
                eprintln!("error: unknown flag {other}; {USAGE}");
                return 64;
            }
            other => entries.push(report::parse_entry(other)),
        }
    }

    let mut out = std::io::stdout().lock();
    if self_check {
        return match selfcheck::run() {
            Ok(check) => {
                let _ = writeln!(
                    out,
                    "{}",
                    report::memnoop_line(
                        check.floor_alloc_bytes,
                        check.floor_alloc_calls,
                        check.floor_live
                    )
                );
                for row in &check.rows {
                    let _ = writeln!(
                        out,
                        "MEMOVERHEAD size={} iters={} counting_ns_per_pair={:.2} \
system_ns_per_pair={:.2} delta_ns_per_pair={:.2}",
                        row.size,
                        row.iters,
                        row.counting_ns,
                        row.system_ns,
                        row.delta_ns()
                    );
                }
                0
            }
            Err(error) => {
                eprintln!("error: self-check failed: {error}");
                70
            }
        };
    }

    // The no-op floor line: what the instrument + process cost
    // before any audit work (printed first on every run — a
    // zero-file run is exactly this line).
    let floor = allocator::snapshot();
    let _ = writeln!(
        out,
        "{}",
        report::memnoop_line(floor.alloc_bytes, floor.alloc_calls, floor.live_bytes())
    );

    // The per-file runs (all printing outside the windows — `walk`
    // returns plain data). `per_class` tracks the DISTINCT file count
    // alongside the reports: repeats multiply runs, never files.
    let mut per_class: Vec<(String, Vec<walk::WalkReport>, usize)> = Vec::new();
    for entry in &entries {
        let path = Path::new(&entry.path);
        for rep in 1..=repeat {
            let report = walk::walk_with(path, demux_only);
            let _ = writeln!(
                out,
                "{}",
                report::memaudit_line(&report, &entry.class, rep, &entry.path)
            );
            if let Some(slot) = per_class
                .iter_mut()
                .find(|(class, _, _)| *class == entry.class)
            {
                slot.1.push(report);
                // The file is new exactly on its first repeat.
                if rep == 1 {
                    slot.2 += 1;
                }
            } else {
                per_class.push((entry.class.clone(), vec![report], 1));
            }
        }
    }

    // The per-class summaries (classes in first-seen order; the
    // ordering is stable and documented — no hidden sorting).
    for (class, reports, distinct) in &per_class {
        let summary = report::summarize(class, reports, *distinct);
        let _ = writeln!(out, "{}", report::memsum_line(&summary));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The class-A fixture walked twice in-process: every packet of
    /// both streams decodes (the same pin apps/cli's bench test
    /// uses), and the second run's peak is dimension-stable (the
    /// bounded-memory property this instrument exists to prove —
    /// tolerance 64 KiB for allocator-internal reuse variance).
    #[test]
    fn fixture_walks_twice_with_stable_peak() {
        let _serial = testutil::serial();
        let path = format!(
            "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
            env!("CARGO_MANIFEST_DIR")
        );
        let first = walk::walk(Path::new(&path));
        assert_eq!(first.verdict, walk::Verdict::Decoded);
        assert!(first.v_pkts > 0, "video packets demuxed");
        assert!(first.a_pkts > 0, "audio packets demuxed");
        assert_eq!(first.v_dec, first.v_pkts, "every video packet decoded");
        assert_eq!(first.a_dec, first.a_pkts, "every audio packet decoded");
        assert_eq!(first.container, "avi");
        let second = walk::walk(Path::new(&path));
        assert_eq!(second.verdict, walk::Verdict::Decoded);
        assert_eq!(
            (second.v_pkts, second.a_pkts, second.v_dec, second.a_dec),
            (first.v_pkts, first.a_pkts, first.v_dec, first.a_dec),
            "the walk is deterministic in counts"
        );
        let peak1 = first.open.peak_live.max(first.walk.peak_live);
        let peak2 = second.open.peak_live.max(second.walk.peak_live);
        assert!(
            peak1.abs_diff(peak2) <= 64 * 1024,
            "second-run peak {peak2} vs first {peak1} — dimension stability"
        );
        // The streaming property: peak growth over baseline stays
        // far below the file size for this fixture (425,516 B).
        let growth = second.walk.growth();
        assert!(
            growth < second.size,
            "walk growth {growth} must stay under file size {}",
            second.size
        );
    }

    /// A garbage file is a typed refusal (the probe's answer), not a
    /// panic and not a decoded row.
    #[test]
    fn garbage_file_is_a_typed_refusal() {
        let _serial = testutil::serial();
        let path = std::env::temp_dir().join("lumen-mem-audit-garbage.bin");
        std::fs::write(&path, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64))
            .unwrap_or_default();
        let report = walk::walk(&path);
        std::fs::remove_file(&path).ok();
        assert_eq!(report.verdict, walk::Verdict::Refused);
        assert!(report.error.is_some(), "the typed error line exists");
        assert_eq!(report.fed_packets(), 0);
    }
}
