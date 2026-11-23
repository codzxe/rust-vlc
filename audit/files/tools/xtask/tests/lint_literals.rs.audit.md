# AUDIT: tools/xtask/tests/lint_literals.rs
| Field | Value |
|---|---|
| Auditor | W01-A04 |
| Date | 2025-11-23 |
| Lines / functions | 436 / 8 helpers + 7 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (integration test; runs via `cargo test -p xtask` / baseline full workspace run) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `target/debug/deps/lint_literals-98275443524d7b11` -> 7 passed; 0 failed; 0 ignored (my run); baseline cargo-test-workspace.txt:6836-6847 identical |

## 1. One-paragraph verdict
Genuine end-to-end acceptance tests, not theater: every test builds a scratch workspace under the system temp dir and runs the REAL xtask binary via `env!("CARGO_BIN_EXE_xtask")` (L56) with the `--root` fixture seam, then asserts exact exit codes (0/1/2), exact violation counts, exact `file:line: kind` report lines, and printed allowlist reasons. I executed the prebuilt test binary myself: 7 passed, 0 failed, 0 ignored. The assertions are mutation-sensitive (exact line numbers 3/4/5/6, "FAIL — 4 violation(s)", "PASS — 0 violations, 3 allowlisted", category totals) — a broken classifier, line tracker, marker parser, mod-file pull, tests-dir exemption, config-layer exemption, or shrink guard would each fail a specific test. The only defects are two coverage gaps the suite shares with the implementation's own blind spots (see lint_literals.rs audit F-001/F-002).

## 2. What actually works (verified)
- `target/debug/deps/lint_literals-98275443524d7b11` -> `test result: ok. 7 passed; 0 failed; 0 ignored` (my execution, exit 0).
- Cross-checked against baseline cargo-test-workspace.txt:6836-6847 (same 7 ok).
- Independently re-verified the behaviors these tests assert, by driving the xtask binary on my own fixtures (see lint_literals.rs audit §2): exit-code contract, marker printing, loopback-in-test-context exemption, C1/C2 scope split — all match what the tests assert.

## 3. Findings
F-001 [S4]
Where: tools/xtask/tests/lint_literals.rs:183-197
Code:
```
    // Production crate: one marked identity site, one inline cfg(test)
    // loopback URL, and a file-level `#[cfg(test)] mod helper;` pull.
```
What is wrong: the exemption test covers only the plain `#[cfg(test)] mod helper;` form; no test seeds `#[cfg(test)] pub mod helper;` — the exact form the implementation mis-lexes (lint_literals.rs:1036 `tokens.first() == Some(&"mod")` rejects the `pub` prefix) and the exact form that exists in the real tree (apps/web/src/io/mod.rs:67 `pub mod mock_fetch;`). The suite shares the implementation's blind spot, so the defect is invisible to CI. What correct looks like: one fixture line with `pub mod helper2;` and an assertion that its loopback URL is allowlisted.
Confidence: HIGH — Verification: read + ran (my fixture proved the impl misses it; rg proved the in-tree form exists).

F-002 [S4]
Where: tools/xtask/tests/lint_literals.rs:146-177
Code:
```
        \"Welcome to Lumen\",\n        \"https://cdn.example.org/manifest.m3u8\",\n        \"/home/alice/video.mkv\",\n        \"#4f8cff\",\n
```
What is wrong: the seeded-literals test only plants bare string literals; no test seeds a literal inside an attribute (`#[doc = "Lumen"]`, `#[command(name = "Lumen")]`), which `capture_attribute` swallows whole (lint_literals.rs:1195-1213) — I demonstrated with a fixture that such a literal is invisible to the gate. No test can fail for a hole nothing probes. Blast radius: latent (zero in-tree instances, rg-verified).
Confidence: HIGH — Verification: ran (fixture demonstrating the blind spot) + read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Must trait + impls | 25-46 | REAL | — | test-local `unreachable!` shim; setup failures fail loudly |
| RunOutcome | 48-52 | REAL | — | captures code/stdout/stderr |
| run_tool | 55-70 | REAL | — | runs the REAL binary (`CARGO_BIN_EXE_xtask`); spawn failure -> code None -> assertions fail loudly |
| Fixture::new | 78-90 | REAL | — | pid + name-scoped temp dir; pre-cleans stale dir |
| Fixture::minimal | 94-98 | REAL | — | crates/engine/src + apps/host/src (the scan domain) |
| Fixture::product_toml_full | 101-129 | REAL | — | all 10 identity fields, real values |
| Fixture::file | 133-137 | REAL | — | path writer |
| Drop for Fixture | 140-144 | REAL | — | `let _ = remove_dir_all` — swallowed cleanup error, acceptable for temp dirs |
| seeded_production_literals_fail_with_file_line_kind | 146-177 | REAL | — | all 4 kinds, exact lines+kinds, exit 1 |
| audited_exemptions_pass_clean_and_are_printed | 179-250 | REAL | S4 | strongest test: marker reason verbatim, 2 loopback reasons, category totals, C2 re-run; misses `pub mod` (F-001) |
| c2_adds_technical_ids_and_accent_that_c1_leaves_alone | 252-323 | REAL | — | GUID C1-pass/C2-fail; accent dual-kind; exact needles |
| url_rules_are_strict_about_test_context_and_hosts | 325-354 | REAL | — | both directions: loopback-in-prod fires, non-loopback-in-test fires |
| a_marker_without_a_reason_suppresses_nothing | 356-378 | REAL | — | reasonless marker is a comment |
| a_broken_identity_source_is_an_xtask_error | 380-418 | REAL | — | missing toml -> 2; incomplete toml names identity.short_name |
| a_shrinking_scan_domain_is_an_error_not_a_smaller_pass | 420-436 | REAL | — | missing apps/ -> exit 2, names the base |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seeded_production_literals_fail_with_file_line_kind | YES | exact 4-count + per-line kind needles; any classifier/line bug breaks it |
| audited_exemptions_pass_clean_and_are_printed | YES | exit 0 + "3 allowlisted" + verbatim reason + exactly-2 loopback reasons + totals; breaks if any exemption regresses |
| c2_adds_technical_ids_and_accent_that_c1_leaves_alone | YES | scope discrimination both directions with exact detail strings |
| url_rules_are_strict_about_test_context_and_hosts | YES | both misdirections fire; exact 2-count and line numbers |
| a_marker_without_a_reason_suppresses_nothing | YES | reasonless marker must NOT suppress |
| a_broken_identity_source_is_an_xtask_error | YES | exit 2 (not fake pass) + field named |
| a_shrinking_scan_domain_is_an_error_not_a_smaller_pass | YES | domain-shrink guard exercised |
No tautologies, no mocks of the tested thing (the real binary is spawned), no #[ignore], no goldens generated by the code under test. 0 ignored per my run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runs the real xtask binary with `--root <scratch>`" (L13) | module doc | REAL | `env!("CARGO_BIN_EXE_xtask")` L56; binary executed by me |
| "Nothing here touches the checked-in tree" (L14-15) | module doc | REAL | all writes under `std::env::temp_dir()`; `--root` passed to every run_tool call |
| "the audited exemptions … pass clean AND are printed" (L4-6) | module doc | REAL | audited_exemptions test asserts printed reasons |

## 7. Dead / duplicated / unreachable code
None. `Must` trait duplicated from testkit into this file and into lint_literals.rs's unit tests (three copies in the workspace) — acceptable test-local pattern, noted for hygiene only.

## 8. Dependencies & flags
Std-only integration test; depends on the xtask bin target (CARGO_BIN_EXE_xtask). No features, no dev-dependencies.

## 9. Open questions for the next agent
None. The two S4 gaps are actionable in one fixture line each; everything else is verified.
