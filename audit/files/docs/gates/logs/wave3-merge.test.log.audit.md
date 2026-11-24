# AUDIT: docs/gates/logs/wave3-merge.test.log (data audit; parts 1-4 = one audit)
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / sections | 2626 / 121 `Running` suites + doctest sections (146 `test result:` lines) |
| File verdict | REAL (historical snapshot) — NOT evidence for current tree state |
| Reachable from shipped entry point? | N/A (log data, not code) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | grep/sed/awk sums (below), all exit 0 |

## 1. One-paragraph verdict
The log is an internally consistent `cargo test` capture: sums of every `test result:` line give **1718 passed / 0 failed / 20 ignored** over 121 test binaries, exactly matching the numbers its citing docs claim. But it is a stale snapshot of a smaller tree: the current workspace baseline records 4744 passed / **2 failed** / 51 ignored, and the exact two tests that fail today — `final_frame_is_current` and `committed_transcripts_match_fresh_runs` — are recorded as `ok` in this log. The log proves nothing about the current tree; any doc citing it as present-tense gate evidence is misleading.

## 2. What actually works (verified)
— `grep -o "test result: ok\. [0-9]* passed" | awk` -> total passed **1718**; `grep -o "[0-9]* ignored" | awk` -> **20**; `grep -c "test result:"` -> 146; `grep -c "FAILED"` -> **0**. Internally consistent arithmetic.
— `sed -n '799,809p'` -> evidence.rs suite block quoted below, byte-verbatim.

## 3. Findings
F-001 [S3] — Stale evidence: currently-failing tests shown as passing.
Where: docs/gates/logs/wave3-merge.test.log:803-808
Code:
```
running 4 tests
test final_frame_is_current ... ok
test ledger_matches_the_artifacts ... ok
test regen ... ignored, rewrites the committed evidence artifacts
test committed_transcripts_match_fresh_runs ... ok

test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.76s
```
What is wrong: The orchestrator baseline (audit/runs/baseline/cargo-test-workspace.txt:2764,2767) records both `committed_transcripts_match_fresh_runs ... FAILED` and `final_frame_is_current ... FAILED`, with `test result: FAILED. 1 passed; 2 failed; 1 ignored` (line 2791). The committed first-vertical evidence artifacts have drifted from what the current code produces. Why it matters: any "tests pass" claim citing this log is false for the current tree. Correct: re-capture the gate log or date-stamp/commit-pin every gate log. Confidence: HIGH — Verification: read + baseline cross-check.

F-002 [S4] — No provenance: the log has no command line, no date, no commit hash, no rustc/cargo version.
Where: docs/gates/logs/wave3-merge.test.log:1
Code:
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.03s
```
What is wrong: A gate log with zero provenance cannot be tied to a tree state; that is exactly how the drift in F-001 went unnoticed. Confidence: HIGH — Verification: read (head/tail inspected).

## 4. Claim cross-references (who cites this log)
| Claim (verbatim, <=25 words) | Source | Verdict | Evidence |
|---|---|---|---|
| "FINAL GATE: PASS — fmt CLEAN / clippy CLEAN / tests 1718 passed / 0 failed / 20 ignored" | docs/worklog/archive-2026-10.md:7694 | REAL for the snapshot, FALSE for the current tree | baseline cargo-fmt.txt shows a real diff (fmt FAILED); baseline test log shows 2 failed |
| "1718 passed / 0 failed / 20 ignored" | docs/gates/phase3-exit-draft.md:110 | REAL (arithmetic verified in-log) | sums above |
| "cargo test --workspace --all-features" produced this log | docs/gates/phase3-exit-draft.md:110 | UNVERIFIED — log does not record its own command line | F-002 |

## 5. Suite identity notes
Biggest suite at part4 boundary: line 2408 `test result: ok. 101 passed; ... finished in 12.02s` (lumen CLI `cmd` suite, incl. `full_tree_is_byte_stable_across_two_runs`). first_vertical integration suites: `Running tests/evidence.rs` at L799, `tests/journey.rs` at L811+, `tests/recovery_matrix.rs` L825+ — all ok here; recovery_matrix and class_b still pass in baseline (only evidence.rs fails).

## 6. Open questions for the next agent
- Who re-runs the wave gates, and are gate logs ever invalidated when the tree changes? No mechanism found in-repo.
