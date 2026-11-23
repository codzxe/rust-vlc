# AUDIT: tools/xtask/tests/feature_audit.rs
| Field | Value |
|---|---|
| Auditor | W01-A06 |
| Date | 2026-11-23 |
| Lines / functions | 222 / 6 helpers + 8 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test code; runs via `cargo test -p xtask --test feature_audit`, part of the workspace test suite the baseline ran: 4744 passed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p xtask --test feature_audit` -> 8 passed, 0 failed (exit 0); `sed -n` quote verification incl. `od -c` on line 201 |

## 1. One-paragraph verdict
These are NOT tautologies — they are real behavioral acceptance tests: each spawns the actual compiled xtask binary (`CARGO_BIN_EXE_xtask`), builds a scratch workspace in the system temp dir, seeds a failure, and asserts BOTH the exit code (0/1/2) and the diagnostic substring. I executed all 8: pass. Mutation reasoning: deleting the B check breaks `implemented_without_evidence_fails`, deleting check C breaks `evidence_citing_missing_file_fails`, deleting the R3 vocabulary/column checks breaks tests 5/6, breaking parse-error handling breaks test 7, changing the unreadable-matrix exit path breaks test 8. So the suite proves the tool works for exactly what the header promises (header lines 2-5 enumerate the seeded failure modes honestly — no overpromise). Two real criticisms: (1) coverage — 5 of the tool's rule families have ZERO committed regression protection anywhere (unit or integration): R1 duplicate-ids, R2 priority/phase range, R5 blocked-without-reason, R6 partial-without-notes, R9 meta/schema_version shape; deleting any of those checks from feature_audit.rs lets the whole suite stay green; even `valid_id`'s wiring into run() is untested (the unit test calls the function directly). (2) The positive fixture's evidence file is a zero-byte file (`fs::write(..., "")`, line 65) — the suite's very definition of "resolvable evidence" is empty-file existence, codifying the existence-only ceiling documented in the source audit (feature_audit.rs F-001).

## 2. What actually works (verified)
— verified by `cargo test -p xtask --test feature_audit` -> `test result: ok. 8 passed; 0 failed` (real binary, real temp-dir fixtures, exit codes + message substrings asserted).

## 3. Findings
F-001 [S3]
Where: tools/xtask/tests/feature_audit.rs:75-95 (GOOD_MATRIX), whole file
Code:
```
const GOOD_MATRIX: &str = r#"
[meta]
schema_version = 1

[[feature]]
id = "container-x"
```
What is wrong: no fixture anywhere in the committed suite exercises R1 duplicate ids, R2 bad priority/phase_target, R5 (blocked without blocked_reason), R6 (partial without notes), or the R9 meta checks (missing [meta], non-int schema_version) — GOOD_MATRIX is entirely well-formed except the seeded mutations. Mutation reasoning (no edit made): deleting the R5 or R6 blocks from src/cmd/feature_audit.rs:240-252, or the R2 check at :192-210, or the duplicate-id check at :187-189 leaves all 12 committed tests (4 unit + 8 integration) green. The tool header claims check A covers "R4/R5/R6 conditionals" — two of those three are regression-unprotected. Why it matters: this tool is the parity-ledger auditor; silent regression of half its structural rules would be invisible. What correct looks like: one seeded-failure test per rule family (the pattern is already established — each is ~10 lines).
Confidence: HIGH — Verification: read + ran (suite green today; behavior of R5/R6 confirmed working only via MY manual synthetic run, not by any committed test).
F-002 [S4]
Where: tools/xtask/tests/feature_audit.rs:63-66
Code:
```
    fn add_evidence_file(&self) {
        must(fs::create_dir_all(self.root.join("crates/demux/src")));
        must(fs::write(self.root.join("crates/demux/src/x.rs"), ""));
    }
```
What is wrong: the canonical "good evidence" of the acceptance suite is a 0-byte file — the suite pins evidence semantics to bare existence (see source-file F-001). Not a test bug (it faithfully tests what the tool guarantees); it documents the weakest possible evidence standard as the accepted one.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: tools/xtask/tests/feature_audit.rs:113-126
Code:
```
    assert!(
        out.stdout.contains("5=1"),
        "per-phase count missing:\n{}",
        out.stdout
    );
```
What is wrong: the counts test asserts phase `5=1` but not phase `4=1` (GOOD_MATRIX row 2 has phase_target 4), and asserts `container=1`/`ui-screen=1` but no status-cell counts — half-hearted assertions on the derived-counts feature.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file is 222 loc; all items listed)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| must | 18-23 | REAL | — | fixture helper; panic-on-setup-failure (correct: broken fixture = broken test) |
| run_audit | 30-43 | REAL | — | spawns real binary; captures code + stdout |
| Fixture::new / write_matrix / add_evidence_file / Drop | 50-73 | REAL | S4 | self-cleaning temp fixture; add_evidence_file = F-002 |
| good_matrix_passes | 97-110 | REAL | — | exit 0 + "result: PASS" |
| good_matrix_reports_derived_counts | 112-126 | REAL | S4 | F-003 |
| implemented_without_evidence_fails | 128-143 | REAL | — | B check pinned |
| evidence_citing_missing_file_fails | 145-162 | REAL | — | C check pinned; negative control (no crates/ tree) |
| invalid_status_value_fails | 164-181 | REAL | — | R3 vocabulary pinned |
| missing_platform_column_fails | 183-196 | REAL | — | R3 column-set pinned |
| unparseable_matrix_fails_with_exit_one | 198-210 | REAL | — | parse-failure -> exit 1, `parse:` line |
| missing_matrix_is_xtask_error | 212-222 | REAL | — | unreadable matrix -> exit 2 (main.rs error mapping) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| good_matrix_passes | YES | real binary; false positives or crashes would flip exit code |
| good_matrix_reports_derived_counts | YES (weakly) | only half the counts asserted (F-003) |
| implemented_without_evidence_fails | YES | seeded B violation; exit 1 + message pinned |
| evidence_citing_missing_file_fails | YES | seeded dangling path; exit 1 + path named |
| invalid_status_value_fails | YES | seeded "done" status |
| missing_platform_column_fails | YES | seeded web-column removal |
| unparseable_matrix_fails_with_exit_one | YES | seeded unterminated `[meta` header (verified via od: literal `"[meta\nschema_version = 1\n"`) |
| missing_matrix_is_xtask_error | YES | no matrix written; exit 2 pinned |
No tautologies, no mocks-of-the-thing-tested, no #[ignore], no cfg-gating. Answer to the assignment question: the tests PROVE the tool works for the 8 behaviors covered; they are silent on 5 rule families (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a good matrix passes, and each seeded failure mode — implemented-without-evidence, dangling evidence path, invalid status value, missing platform column, unparseable TOML — fails with exit 1" | tests/feature_audit.rs:1-5 | REAL | all 8 executed, pass; the enumerated list is exactly what is seeded (no overpromise) |
| "the real binary via CARGO_BIN_EXE_xtask, nothing touches the checked-in tree" | tests/feature_audit.rs:7-9 | REAL | read: Command::new(env!("CARGO_BIN_EXE_xtask")); fixtures under std::env::temp_dir with PID-unique names, Drop-cleanup |

## 7. Dead / duplicated / unreachable code
None. All helpers used; GOOD_MATRIX reused across tests via .replace() mutations (reasonable, not duplication).

## 8. Dependencies & flags
Test-only: std (fs, path, process). No dev-dependencies (the `must` helper is re-implemented locally with the rationale at lines 15-17: testkit is not an xtask dev-dep because boundaries.toml dev_deps are Lead-owned). No feature flags.

## 9. Open questions for the next agent
- If R5/R6/R2/R1-dup/R9-meta regression tests are added, they belong here following the established pattern; the source audit's F-002 (undisclosed rule gaps) should be resolved in the same change.
