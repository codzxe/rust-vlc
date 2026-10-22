//! Randomized real-media corpus suite — the probe surface (ED-FIX-5; the
//! user's 2026-10-22 randomized-corpus directive: "real audio and video
//! generated from script, random formats, too many samples, real numbers").
//!
//! This suite is the anti-echo-chamber instrument the directives audit
//! demanded (D-5): the probe runs against **externally generated media**
//! (ffmpeg, randomized parameters, recorded in the corpus manifest), not
//! against fixtures authored by the same hand that wrote the probe. The
//! ground truth (true container per file) comes from ffprobe — an external
//! oracle — and is handed in by the harness driver as a TSV case list.
//!
//! The contract here is deliberately asymmetric with the fixture suites:
//! this test **records** what the probe says (verdict, format, confidence,
//! bytes examined, latency) and **fails only on contract violations** —
//! a panic, a hang, or a missing corpus when the harness asked for one.
//! Verdict *quality* (identification accuracy, garbage rejection rate) is
//! judged by the aggregator against the oracle, and lands in the report as
//! measured numbers.
//!
//! Activation (the harness driver runs exactly this):
//!
//! ```text
//! LUMEN_CORPUS=<dir> LUMEN_CORPUS_CASES=<tsv> LUMEN_CORPUS_RESULTS=<tsv> \
//!   cargo test --release -p media-probe --test corpus -- --ignored --nocapture
//! ```
//!
//! Without the env trio the suite is a no-op (green) — the local gate
//! never depends on a generated corpus existing.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use media_probe::{ProbeConfig, ProbeRegistry, probe_stream};
use media_types::EngineError;
use testkit::Must;

/// Open a corpus file through the real access stack (the same
/// `FileAccess` → `SourceStream` composition the engine's open path
/// runs — card 2.A1's seam, not an in-memory stand-in).
fn open_via_input(
    path: &std::path::Path,
) -> Result<Box<dyn plugin_api::SourceStream>, EngineError> {
    let text = match path.to_str() {
        Some(text) => format!("file://{text}"),
        None => unreachable!("corpus paths are UTF-8 ({path:?})"),
    };
    let uri = match media_types::ids::SourceUri::parse(&text) {
        Ok(uri) => uri,
        Err(err) => unreachable!("corpus path is a valid file URI ({path:?}): {err}"),
    };
    input_shim::open(&uri)
}

/// The `input` crate is a dev-dependency; this alias keeps the open
/// path visible at the call site without a glob import.
mod input_shim {
    use media_types::EngineError;
    use media_types::ids::SourceUri;
    use plugin_api::Access;

    pub fn open(uri: &SourceUri) -> Result<Box<dyn plugin_api::SourceStream>, EngineError> {
        let access = input::FileAccess::new();
        access.open(uri)
    }
}

/// One row of the harness case list (TSV; fields carry no tabs by
/// construction of the generator).
struct ProbeCase {
    rel: String,
    truth_format: String,
    class: String,
}

fn parse_cases(text: &str) -> Vec<ProbeCase> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let mut fields = line.split('\t');
        let (Some(rel), Some(truth), Some(class)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        out.push(ProbeCase {
            rel: rel.to_owned(),
            truth_format: truth.to_owned(),
            class: class.to_owned(),
        });
    }
    out
}

/// Probe one file and return the result row fields (verdict, format,
/// confidence, bytes examined, error code, elapsed µs).
#[allow(clippy::type_complexity)]
fn probe_one(path: &std::path::Path) -> (String, String, u32, u64, String, u128) {
    let t0 = Instant::now();
    let mut stream = match open_via_input(path) {
        Ok(stream) => stream,
        Err(err) => {
            return (
                "open-error".to_owned(),
                String::new(),
                0,
                0,
                err.context.code.to_owned(),
                t0.elapsed().as_micros(),
            );
        }
    };
    let registry = ProbeRegistry::with_builtins();
    let config = ProbeConfig::default();
    match probe_stream(&registry, &mut *stream, &config) {
        Ok(report) => (
            "probed".to_owned(),
            report.format.as_str().to_owned(),
            u32::from(report.confidence.as_u8()),
            report.bytes_examined as u64,
            String::new(),
            t0.elapsed().as_micros(),
        ),
        Err(err) => (
            "rejected".to_owned(),
            String::new(),
            0,
            0,
            err.context.code.to_owned(),
            t0.elapsed().as_micros(),
        ),
    }
}

/// The env-gated corpus run. `#[ignore]`d: only the harness driver
/// activates it (the gate must not depend on a generated corpus).
#[test]
#[ignore = "randomized-corpus harness: run via tools/corpus/run_harness.py"]
fn corpus_probe_records_every_file() {
    let (Some(corpus), Some(cases_path), Some(results_path)) = (
        std::env::var("LUMEN_CORPUS").ok(),
        std::env::var("LUMEN_CORPUS_CASES").ok(),
        std::env::var("LUMEN_CORPUS_RESULTS").ok(),
    ) else {
        eprintln!("corpus: LUMEN_CORPUS env trio not set — no-op");
        return;
    };
    let corpus = PathBuf::from(corpus);
    let cases = parse_cases(&std::fs::read_to_string(&cases_path).must());
    assert!(
        !cases.is_empty(),
        "harness handed an empty case list ({cases_path:?})"
    );
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&results_path)
        .must();
    writeln!(out, "rel\tclass\ttruth_format\tverdict\tformat\tconfidence\tbytes_examined\terr_code\telapsed_us").must();
    for case in &cases {
        let path = corpus.join(&case.rel);
        // A missing corpus file is a harness contract violation, not a
        // media verdict — fail loudly so the driver surfaces it.
        assert!(path.is_file(), "corpus file missing: {path:?}");
        let (verdict, format, confidence, examined, err, elapsed) = probe_one(&path);
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            case.rel,
            case.class,
            case.truth_format,
            verdict,
            format,
            confidence,
            examined,
            err,
            elapsed
        )
        .must();
    }
    eprintln!(
        "corpus(probe): {} cases recorded -> {results_path:?}",
        cases.len()
    );
}
