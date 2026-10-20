//! Binary-surface integration tests for xtask v0 (card 1.A3).
//!
//! These run the compiled xtask binary with cheap invocations only. They
//! NEVER execute the real gate table (too slow — doc 10 §2; the full gate is
//! verified live by the card's VERIFY step and by CI). The stub and dry-run
//! behaviors exercised here are the frozen interface other cards rely on.

use std::path::PathBuf;
use std::process::Command;

struct RunOutcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_xtask(args: &[&str]) -> RunOutcome {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.args(args);
    match cmd.output() {
        Ok(out) => RunOutcome {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(err) => RunOutcome {
            code: None,
            stdout: String::new(),
            stderr: format!("failed to spawn xtask binary: {err}"),
        },
    }
}

/// The Lumen workspace root (this test file lives in `tools/xtask/tests`).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn help_exits_zero_and_lists_the_frozen_subcommand_table() {
    let out = run_xtask(&["help"]);
    assert_eq!(out.code, Some(0), "stderr: {}", out.stderr);
    for name in [
        "gate",
        "schema-gen",
        "tokens",
        "codegen",
        "check-architecture",
        "rebrand",
    ] {
        assert!(
            out.stdout.contains(name),
            "help output missing `{name}`:\n{}",
            out.stdout
        );
    }
    assert!(out.stdout.contains("card 1.A3"));
}

#[test]
fn no_args_is_a_usage_error() {
    let out = run_xtask(&[]);
    assert_eq!(out.code, Some(2), "stderr: {}", out.stderr);
    assert!(out.stderr.contains("Usage"), "stderr: {}", out.stderr);
}

#[test]
fn unknown_subcommand_is_a_usage_error_listing_known_ones() {
    let out = run_xtask(&["definitely-not-a-subcommand"]);
    assert_eq!(out.code, Some(2), "stderr: {}", out.stderr);
    assert!(
        out.stderr
            .contains("unknown subcommand `definitely-not-a-subcommand`"),
        "stderr: {}",
        out.stderr
    );
    assert!(out.stderr.contains("gate"), "stderr: {}", out.stderr);
}

#[test]
fn pre_registered_stubs_fail_honestly_with_their_card() {
    // `tokens` graduated when card 1.B4 landed; `schema-gen` when 1.B2
    // landed (both Lead-completed salvages); `rebrand` when 1.E4 landed
    // (Lead-implemented after two agent context-deadlines): they run the
    // real pipelines.
    for (sub, card) in [("codegen", "1.D4"), ("check-architecture", "1.E1")] {
        let out = run_xtask(&[sub]);
        assert_eq!(out.code, Some(2), "`{sub}` stderr: {}", out.stderr);
        assert!(
            out.stderr
                .contains(&format!("`{sub}` is not yet implemented (card {card})")),
            "`{sub}` stderr: {}",
            out.stderr
        );
    }
}

// Card 1.E4: the rebrand acceptance skeleton lives at the repo root
// (tests/rebrand/, next to its fixture) per doc 07 §8.3; the root
// manifest is virtual, so it is compiled into THIS test binary via a
// #[path] include (the skeleton's header documents the wiring).
#[path = "../../../tests/rebrand/rebrand_skeleton.rs"]
mod rebrand_skeleton;

#[test]
fn gate_bad_flag_is_a_usage_error() {
    let out = run_xtask(&["gate", "--no-such-flag"]);
    assert_eq!(out.code, Some(2), "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("unexpected argument `--no-such-flag`"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn gate_dry_run_executes_nothing_and_reports_the_full_table() {
    let out = run_xtask(&["gate", "--dry-run"]);
    assert_eq!(out.code, Some(0), "stderr: {}", out.stderr);
    assert!(
        out.stdout.contains("=== LUMEN GATE REPORT ==="),
        "stdout:\n{}",
        out.stdout
    );
    // The doc 10 §2.1 table is always printed in full — dry-run proves the
    // shape without executing anything.
    assert!(out.stdout.contains("1 fmt"), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout.contains("10 wasm-pack"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("result: DRY RUN"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("full log: target/gate-report.json"),
        "stdout:\n{}",
        out.stdout
    );
    // Steps whose target does not exist yet are honestly SKIPped. The ui
    // steps activate on `ui/package.json` (the pnpm-project marker), not the
    // bare `ui/` dir — config stubs from card 1.B3 must not flip the gate.
    if !repo_root().join("ui/package.json").is_file() {
        assert!(
            out.stdout.contains("SKIP (ui/package.json not present)"),
            "stdout:\n{}",
            out.stdout
        );
    }
    // The JSON log is always written.
    assert!(
        repo_root().join("target/gate-report.json").is_file(),
        "gate-report.json was not written"
    );
}

#[test]
fn gate_dry_run_json_prints_the_envelope() {
    let out = run_xtask(&["gate", "--dry-run", "--json"]);
    assert_eq!(out.code, Some(0), "stderr: {}", out.stderr);
    assert!(
        out.stdout.contains("\"schema\": \"evidence/1\","),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("\"status\": \"dry\","),
        "stdout:\n{}",
        out.stdout
    );
}
