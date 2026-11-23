# AUDIT: tools/xtask/tests/check_panics.rs
| Field | Value |
|---|---|
| Auditor | W01-A04 |
| Date | 2025-11-23 |
| Lines / functions | 209 / 7 helpers + 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (integration test; runs via `cargo test -p xtask`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `target/debug/deps/check_panics-bd5556924084c4c8` -> 4 passed; 0 failed; 0 ignored (my run); `./target/debug/xtask check-panics` (real tree) -> exit 1, 2 violations (cross-evidence); baseline cargo-test-workspace.txt:6768-6777 |

## 1. One-paragraph verdict
Honest fixture-based acceptance tests: each spawns the REAL xtask binary (`env!("CARGO_BIN_EXE_xtask")`, L52) on a scratch workspace with `--root`, asserts exact exit codes and report substrings, and — the strong part — includes an adversarial anti-blanket-pass test (right path, wrong citation message must still FAIL) and a scope-shrink guard test. I ran the prebuilt test binary myself: 4/4 pass. The tests prove the gate's classifier, sanctioned zones, and citation scoping work. What they cannot prove — and demonstrably do not cover — is the gate's relationship to the real tree: I independently ran `xtask check-panics` on the real workspace and it FAILS with 2 live violations (crates/demux/src/ts/mod.rs:369,372 `unreachable!` in non-test code), a state invisible to every gate lane because nothing wires check-panics (already filed as W01-A10's F-001/F-002 on the gate source; my run reproduces their result exactly — same 2 sites, exit 1). Three minor test-suite gaps below.

## 2. What actually works (verified)
- `target/debug/deps/check_panics-bd5556924084c4c8` -> `test result: ok. 4 passed; 0 failed; 0 ignored` (my execution).
- Baseline identical: cargo-test-workspace.txt:6768-6777.
- `./target/debug/xtask check-panics` on real tree -> `FAIL — 2 violation(s) outside the sanctioned zones: crates/demux/src/ts/mod.rs:369 / :372` — the behaviors the tests assert (seeded violation exit 1 + site naming; sanctioned zones exit 0; wrong-message citation exit 1; missing crate exit 2) are all real behaviors of a real tool; the suite is not asserting against a mock.
- Verified the fixture's 9-crate list (L90-100) matches the implementation's `SCANNED_CRATES` (check_panics.rs:77-87) name-for-name, and that `crates/input/src/contract.rs` + the `pipeline.rs`/`"fresh SharedBytes has refcount 1"` citation the tests rely on exist verbatim in `CONTRACT_MODULES`/`CITED_INVARIANTS` (check_panics.rs:93, 107-114).

## 3. Findings
F-001 [S4]
Where: tools/xtask/tests/check_panics.rs:89-103
Code:
```
    /// Create `crates/<name>/src` for all nine scanned crates.
    fn all_nine(&self) {
        for name in [
            "playback-engine",
```
What is wrong: the fixture pins the scan domain in the ADDITION direction only. The impl errors if a crate in ITS list is missing (so adding a 10th crate to SCANNED_CRATES breaks these tests — good), but REMOVING a crate from the impl list is silent: the impl iterates only its own list, extra fixture dirs are ignored, and no assertion checks the report's domain count ("across 9 crates"). A crate quietly leaving the panic gate's domain would not fail this suite. What correct looks like: assert the report line's crate count (e.g. `contains("9 crates")`).
Confidence: HIGH — Verification: read (impl loop at check_panics.rs:317-322) + ran (real-tree report prints the domain count, unasserted).

F-002 [S4]
Where: tools/xtask/tests/check_panics.rs:1-13
Code:
```
//! workspace are caught, the sanctioned zones pass clean, and the
//! cited-invariant allowance is path+message scoped, never a blanket
//! file pass.
```
What is wrong: the doc's claim ("prove the gate's teeth") is true for fixtures, but the suite never asserts anything about the real tree, and the real tree is currently RED (my run: 2 violations, exit 1). Teeth that only ever bite scratch directories, wired into no lane (full_gate.sh/gate.rs/CI have no check-panics step — rg-verified, zero invocations outside tools/xtask), protect nothing. The test file is not the right place for a real-tree assertion (that is the gate lane's job), so this is recorded as a gap pointer, not a test defect; the wiring failure itself is W01-A10 F-001 (their file). Cross-confirmed: my real-tree run reproduces their finding exactly.
Confidence: HIGH — Verification: ran + rg.

F-003 [S4]
Where: tools/xtask/tests/check_panics.rs:138-173
Code:
```
    // The F-2 executable-contract module zone.
    fx.file(
        "input",
        "contract.rs",
```
What is wrong: no fixture seeds a file-level `#[cfg(test)] pub mod helper;` — the shared scanner's mod-decl parser rejects the `pub` prefix (`tokens.first() == Some(&"mod")`, check_panics.rs:634, identical to lint_literals.rs:1036), so such a file's panic sites would be false-positive violations. Zero in-tree instances inside the 9-crate domain (rg: the only `#[cfg(test)] pub mod` decl in the repo is apps/web/src/io/mod.rs:67, outside this gate's scope), so latent only; the suite shares the implementation blind spot either way.
Confidence: HIGH — Verification: read + rg (in-tree census) + ran (demonstrated the same parser defect on the lint-literals side with a fixture).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Must trait + impls | 22-43 | REAL | — | test-local `unreachable!` shim (the sanctioned asymmetry, L19-21) |
| RunOutcome | 45-49 | REAL | — | code/stdout/stderr capture |
| run_checker | 51-66 | REAL | — | spawns the REAL binary; spawn failure -> None -> loud failure |
| Fixture::new | 74-86 | REAL | — | pid+name temp dir, pre-cleaned |
| Fixture::all_nine | 89-103 | REAL | S4 | F-001 one-directional domain pin |
| Fixture::file | 106-110 | REAL | — | path writer |
| Drop for Fixture | 113-117 | REAL | — | swallowed cleanup error (temp dirs, acceptable) |
| seeded_non_test_site_fails | 119-136 | REAL | — | exit 1 + exact `:2:` site; teeth proof |
| sanctioned_zones_pass_clean | 138-173 | REAL | S4 | doc-comment stripping, cfg(test) region, contract module, cited invariant — all exercised; no `pub mod` case (F-003) |
| cited_invariant_allowance_is_scoped_to_the_message | 175-194 | REAL | — | the anti-blanket-pass proof — right path + wrong message must fail |
| a_missing_scanned_crate_is_an_error_not_a_scope_shrink | 196-209 | REAL | — | exit 2, names the missing tree |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seeded_non_test_site_fails | YES | exit 1 + violation count + exact file/line needle |
| sanctioned_zones_pass_clean | YES | exit 0 requires stripping, region tracking, contract-module and citation allowances all working |
| cited_invariant_allowance_is_scoped_to_the_message | YES | any loosening of citation matching to path-only fails it |
| a_missing_scanned_crate_is_an_error_not_a_scope_shrink | YES | shrink guard must error, not pass smaller |
No mocks, no tautologies, no #[ignore] (0 ignored in my run). The adversarial scoping test (L175-194) is the mark of a suite written to resist its own gate being defanged.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runs the real xtask binary with `--root <scratch>`" (L9-10) | module doc | REAL | L52-53; binary executed by me |
| "Nothing here touches the checked-in tree" (L11) | module doc | REAL | all writes under temp_dir |
| "the cited-invariant allowance is path+message scoped, never a blanket file pass" (L4-5) | module doc | REAL | L175-194 proves the negative direction |
| "the nine scanned crates' `src/` trees" (L8) | module doc | REAL | matches SCANNED_CRATES verbatim (checked) |

## 7. Dead / duplicated / unreachable code
None dead. `Must`/`RunOutcome`/`Fixture` are near-copies of tests/lint_literals.rs L25-144 (third copy of `Must` with the xtask unit tests) — test-local duplication, hygiene note only.

## 8. Dependencies & flags
Std-only; depends on the xtask bin target. No features, no dev-deps.

## 9. Open questions for the next agent
- Whoever owns scripts/full_gate.sh / gate.rs: check-panics remains unwired while the tree is red (2 demux violations) — see W01-A10 F-001/F-002; my independent run confirms (same sites, exit 1).
- The panic ratchet (docs/audit/panic-baseline.json) vs check-panics contradictory-verdict issue is W01-A10 F-004; not re-audited here.
