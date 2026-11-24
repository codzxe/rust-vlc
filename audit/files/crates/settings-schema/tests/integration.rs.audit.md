# AUDIT: crates/settings-schema/tests/integration.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 424 / 6 helpers + 8 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file) — it exercises the real `schema-gen` binary via `env!("CARGO_BIN_EXE_schema-gen")` (line 26) and the real committed repo files |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | baseline citation (cargo-test-workspace.txt:5372-5382: 8/8 pass); sed quote verification |

## 1. One-paragraph verdict
This is what integration tests should look like: they spawn the compiled generator binary, assert process exit codes (0 no-drift / 1 drift / 2 validation+IO error), compare two separate runs byte-for-byte for idempotence, prove the duplicate-key fixture fails with an actionable `error[D2]` diagnostic naming the first definition line, prove nothing is written on failed validation, and re-derive every expectation (entry counts, screen counts) from `schemas/settings.toml` via an independent raw `toml::Table` parse instead of pinning literals — the test's own comments document the drift incident (c71cd5a, 27→32 entries) that motivated this. All 8 pass in the baseline. The only defects are two pieces of vestigrial debris in the drift and scratch-dir helpers.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:5372-5382: `test result: ok. 8 passed; 0 failed` including `committed_outputs_have_no_drift`, `seed_shape_is_what_every_committed_artifact_self_reports`, `regeneration_is_idempotent_byte_for_byte`, `duplicate_option_fixture_fails_with_rule_d2`, `stale_output_drift_fails_check_with_exit_one`.
— Mutation reasoning (no edit made): delete the D2 rule from validate.rs -> `duplicate_option_fixture_fails_with_rule_d2` fails at `out.code == Some(2)` (a valid schema would generate exit 0 and write outputs, failing the `!dir.join("out").join("settings.d.ts").is_file()` assert too). Corrupt a committed artifact by hand -> `committed_outputs_have_no_drift` fails with the binary's stdout in the message. Grow the seed without regenerating -> `seed_shape_is_what_every_committed_artifact_self_reports` fails first, naming both regeneration commands (assert message at lines 214-217). These tests bite.

## 3. Findings
F-001 [S4]
Where: crates/settings-schema/tests/integration.rs:342-347
Code:
```
    // Corrupt one committed-looking output, then --check must flag drift.
    let help_path = out_dir.join("settings-help.txt");
    let stale = read(&help_path).replace("27 entries", "999 entries");
    // The fixture only has 2 entries; any hand edit is drift:
    let stale = stale.replace("playback", "playbackx");
    fs::write(&help_path, stale).ok();
```
What is wrong: the first `.replace("27 entries", "999 entries")` is dead code — the 2-entry fixture's help text cannot contain "27 entries", so the replace is a no-op every run (the very next comment admits "The fixture only has 2 entries"). Debris from adapting this test from a real-file variant. The test still passes because the second replace does the corruption. Also `fs::write(...).ok()` swallows a write failure; a failed write would surface only indirectly as a confusing `--check` exit-code assert. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/settings-schema/tests/integration.rs:48-54
Code:
```
/// A unique-enough scratch dir per test-binary run.
fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lumen-schema-gen-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).ok();
    dir
}
```
What is wrong: `create_dir_all(...).ok()` swallows the only error that makes every later assert meaningless; the failure mode is "all tests fail with spawn noise" instead of "could not create scratch dir". Test-only blast radius. Confidence: HIGH — Verification: read.

## 4. Per-function table (helpers; file is a test, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run_bin | 25-41 | REAL | - | spawns CARGO_BIN_EXE; Err arm returns code None — asserts distinguish |
| repo_root | 44-46 | REAL | - | MANIFEST_DIR/../.. — works in workspace |
| temp_dir | 49-54 | REAL | S4 | swallowed create error (F-002) |
| write_fixture | 90-97 | REAL | - | asserts both writes |
| read | 99-104 | REAL | - | returns `<unreadable: ...>` sentinel instead of panicking — feeds asserts |
| committed_seed_entry_count | 132-141 | REAL | - | independent toml::Table parse, no crate code |
| screen_of (inner) | 167-174 | REAL | - | collection-time validation with row-index messages |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_outputs_have_no_drift (107) | YES | real binary `--check` over real committed files; count derived from seed, not pinned |
| seed_shape_is_what_every_committed_artifact_self_reports (151) | YES | ties all three self-reported counts (d.ts header, per-screen help rows) to an independent seed parse; comment documents fixing a previously tautological `screens.len() == entries.len()` assert (lines 178-180) |
| regeneration_is_idempotent_byte_for_byte (237) | YES | two separate process runs, byte-compare + non-empty + self-check exit 0 |
| duplicate_option_fixture_fails_with_rule_d2 (283) | YES | exit 2, rule name, first-definition line, AND no outputs written |
| stale_output_drift_fails_check_with_exit_one (329) | YES | exit 1 + "differs from the regenerated output at line" |
| missing_schema_file_is_an_io_error_not_a_panic (381) | YES | exit 2 + "reading" in stderr (no panic) |
| help_flag_prints_usage_and_exits_zero (399) | YES | pins usage + --check presence |
| committed_generated_files_exist (413) | YES | pins the G1/G2/G3 file contract, non-empty |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "--check on the committed outputs exits 0 (no drift)" | integration.rs:8 | REAL | baseline: test passes (cargo-test-workspace.txt:5382) |
| "regeneration is idempotent (two runs, byte-identical outputs)" | integration.rs:9 | REAL | baseline:5375 passes |
| "a duplicate-option fixture FAILS with an actionable diagnostic naming the rule (D2)" | integration.rs:10-11 | REAL | baseline:5373 passes; asserts rule + line + no-write |
| "drift (a stale committed output) exits 1" | integration.rs:12 | REAL | baseline:5376 passes (though via a temp-dir copy, not a committed file — the header comment says "stale committed output", the body corrupts a temp copy) |

## 7. Dead / duplicated / unreachable code
- Dead `.replace("27 entries", "999 entries")` (F-001).
- No unreachable tests; no #[ignore].

## 8. Dependencies & flags
std only + toml (independent parse). Uses CARGO_BIN_EXE_ compile-time env. No features.

## 9. Open questions for the next agent
- The drift test corrupts a temp-dir copy though its header claims "a stale committed output"; a stricter variant would corrupt-and-restore an actual committed file. Real-path coverage is carried by `committed_outputs_have_no_drift`, which is adequate.
