//! The evidence currency checks (doc 10 §6.4/§7): the committed
//! artifacts under `evidence/` must match a FRESH run — which is both
//! the non-staleness proof for the evidence and the suite-level
//! determinism proof. The `#[ignore]` test regenerates the artifacts;
//! the normal gate only READS them (a gate run never dirties the
//! tree).

use first_vertical::digest::sha256_hex;
use first_vertical::evidence as ev;
use first_vertical::harness::{Labels, normalize_run_specific};
use first_vertical::scenarios::{self, SCENARIO_IDS};

/// Run one scenario, asserting it green first (evidence only speaks
/// for green runs).
fn green(id: &str) -> first_vertical::harness::ScenarioRun {
    let run = scenarios::by_id(id).unwrap_or_else(|reason| {
        unreachable!("scenario dispatch is total over SCENARIO_IDS: {reason}")
    });
    for check in &run.checks {
        assert!(
            check.passed,
            "scenario `{id}` — check failed: {} — {}",
            check.claim, check.detail
        );
    }
    run
}

/// Every committed transcript matches a fresh run byte-for-byte
/// (after normalizing the two per-run label values — `environment_kind`
/// and `os` name the producing machine by design, doc 10 §7.1), and
/// the STABLE labels are asserted semantically: `hardware_class` is
/// `mock` and the devices are the null-audio + software-render pair —
/// on every machine this suite can run on (asserting `real` anywhere
/// would be fabrication).
#[test]
fn committed_transcripts_match_fresh_runs() {
    let labels = Labels::detect();
    assert_eq!(labels.hardware_class, "mock");
    assert_eq!(labels.devices, vec!["null-audio", "software-render"]);
    for id in SCENARIO_IDS {
        let run = green(id);
        let fresh = normalize_run_specific(&run.transcript_bytes());
        let path = ev::transcript_path(id);
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| unreachable!("read {}: {error}", path.display()));
        assert_eq!(
            fresh,
            normalize_run_specific(&committed),
            "the committed transcript for `{id}` drifted from a fresh run — \
             regenerate with: cargo test -p first-vertical --test evidence \
             -- --ignored regen"
        );
        // The stable labels are in the committed artifact verbatim.
        assert!(
            committed.contains("\"hardware_class\": \"mock\""),
            "`{id}`: the committed transcript lacks the mock label"
        );
        assert!(
            committed.contains("\"null-audio\""),
            "`{id}`: the committed transcript lacks the null-audio device label"
        );
        assert!(
            committed.contains("\"software-render\""),
            "`{id}`: the committed transcript lacks the software-render device label"
        );
    }
}

/// The ledger matches the artifacts it names: every recorded path
/// exists, its digest and byte count are the file's actual values,
/// every scenario verdict is pass, the scenario table covers exactly
/// the suite's ids, and the hardware columns carry the honest labels
/// (audio sink = null adapter/mock, video render = software
/// compositor/mock, the 3-OS CI verdict recorded as PENDING).
#[test]
fn ledger_matches_the_artifacts() {
    let ledger_path = ev::ledger_path();
    let ledger = std::fs::read_to_string(&ledger_path)
        .unwrap_or_else(|error| unreachable!("read {}: {error}", ledger_path.display()));
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    // Every transcript artifact: path + digest + length.
    let mut artifacts = 0;
    for id in SCENARIO_IDS {
        let relative = format!("evidence/transcripts/{id}.json");
        let bytes = std::fs::read(crate_dir.join(&relative))
            .unwrap_or_else(|error| unreachable!("read {relative}: {error}"));
        assert!(
            ledger.contains(&format!("\"path\": \"{relative}\"")),
            "the ledger does not name {relative}"
        );
        assert!(
            ledger.contains(&format!("\"sha256\": \"{}\"", sha256_hex(&bytes))),
            "the ledger's digest for {relative} is stale"
        );
        assert!(
            ledger.contains(&format!("\"bytes\": {}", bytes.len())),
            "the ledger's byte count for {relative} is stale"
        );
        artifacts += 1;
    }
    // The final frame.
    let png = std::fs::read(ev::final_frame_path())
        .unwrap_or_else(|error| unreachable!("read final-frame.png: {error}"));
    assert!(
        ledger.contains("\"path\": \"evidence/final-frame.png\""),
        "the ledger does not name the final frame"
    );
    assert!(
        ledger.contains(&format!("\"sha256\": \"{}\"", sha256_hex(&png))),
        "the ledger's final-frame digest is stale"
    );
    assert!(
        ledger.contains(&format!("\"bytes\": {}", png.len())),
        "the ledger's final-frame byte count is stale"
    );
    assert_eq!(artifacts + 1, 13, "12 transcripts + the final frame");

    // The scenario table: exactly the suite's ids, all green.
    assert_eq!(
        ledger.matches("\"verdict\": \"pass\"").count(),
        SCENARIO_IDS.len(),
        "every scenario must be recorded pass"
    );
    assert!(
        !ledger.contains("\"verdict\": \"fail\""),
        "a failed scenario was committed as evidence"
    );
    for id in SCENARIO_IDS {
        assert!(
            ledger.contains(&format!("\"id\": \"{id}\"")),
            "the ledger's scenario table lacks `{id}`"
        );
    }

    // The honest hardware columns (doc 10 §7.1 — mock, never real).
    assert!(ledger.contains("\"hardware_class\": \"mock\""));
    assert!(ledger.contains("\"status\": \"PENDING\""));
    assert!(ledger.contains("null adapter"));
    assert!(ledger.contains("software compositor"));
    // The schema + envelope shape (doc 10 §7.2).
    assert!(ledger.contains("\"schema\": \"evidence/1\""));
    assert!(ledger.contains("\"environment\""));
    assert!(ledger.contains("\"commands\""));
    assert!(ledger.contains("\"labels\""));
}

/// The final-frame PNG is current: a fresh run-to-end produces
/// byte-identical bytes, and the run-to-end transcript's own digest
/// record names the committed file's bytes.
#[test]
fn final_frame_is_current() {
    let run = green("run-to-end");
    let transcript = run.transcript_bytes();
    let png = run
        .png
        .clone()
        .unwrap_or_else(|| unreachable!("run-to-end always captures a frame"));
    let committed = std::fs::read(ev::final_frame_path())
        .unwrap_or_else(|error| unreachable!("read final-frame.png: {error}"));
    assert_eq!(
        png, committed,
        "the committed final-frame.png drifted from a fresh run"
    );
    assert!(
        transcript.contains(&format!("\"sha256\": \"{}\"", sha256_hex(&committed))),
        "the run-to-end transcript's final_png digest does not name the \
         committed frame"
    );
    assert!(
        transcript.contains("\"width\": 320") && transcript.contains("\"height\": 240"),
        "the transcript's final-frame extent is not the media's 320x240"
    );
}

/// Regenerate every committed artifact from a fresh run. NOT part of
/// the gate: this is the writer (the snapshot-regeneration pattern);
/// run it deliberately after an intentional behavior change —
///
/// ```text
/// cargo build -p lumen-cli   # the wire scenario needs the binary
/// cargo test -p first-vertical --test evidence -- --ignored regen
/// ```
///
/// It refuses to write when any scenario check fails (evidence only
/// commits green runs) and it also rewrites the testkit-form text
/// record (`evidence/2.E1-first-vertical.txt`), the human-stable
/// shape the wave-review gate reads.
#[test]
#[ignore = "rewrites the committed evidence artifacts"]
fn regen() {
    let summary = ev::regen_all()
        .unwrap_or_else(|reason| unreachable!("evidence regeneration failed: {reason}"));

    // The text record (testkit's own doc 10 §7 vocabulary).
    let record = testkit::EvidenceRecord::new("2.E1-first-vertical", testkit::HardwareClass::Mock)
        .claim(
            "first-vertical.journey.open",
            "scenario `open` (tests/integration/first-vertical, in-process through lumen_dispatch)",
        )
        .claim(
            "first-vertical.journey.play-pause-seek-stop-run-to-end",
            "scenarios `play`/`pause`/`seek`/`stop`/`run-to-end` (same suite)",
        )
        .claim(
            "first-vertical.class-b.missing-file-trunc-garbage-noidx",
            "scenarios `missing-file`/`corrupt-*` (same suite, full stack)",
        )
        .claim(
            "first-vertical.wire.session",
            "scenario `wire-session` (the binary's --control-stdio NDJSON mode)",
        )
        .claim(
            "first-vertical.evidence.transcripts-and-ledger",
            "evidence/ledger.json + evidence/transcripts/*.json + final-frame.png",
        )
        .claim(
            "first-vertical.ci.3os-green",
            "PENDING — no CI runner has executed the workflows (see ledger ci_3os_green)",
        );
    let text_path = ev::evidence_dir().join("2.E1-first-vertical.txt");
    std::fs::write(&text_path, record.render())
        .unwrap_or_else(|error| unreachable!("write {}: {error}", text_path.display()));
    println!("{summary}");
    println!("rewrote {}", text_path.display());
}
