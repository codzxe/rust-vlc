# AUDIT: docs/gates/logs/merge-3a1-c79ec88.test.log (data audit, parts L1-792/L793-1588/L1589-2028)
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines | 2028 (all 3 parts read via Read/grep/awk) |
| File verdict | REAL-UNPROVEN (genuine cargo-test capture; tree state it proves no longer holds) |
| Reachable from shipped entry point? | N/A — evidence artifact, not code |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | grep/awk/sed/stat/date (exit 0 each) |

## 1. One-paragraph verdict
This is a real `cargo test` workspace log (compile lines, 93 `running N tests`
groups, doc-tests section, per-binary timings). It records 1362 passed /
0 failed / 4 ignored across 107 `test result:` groups at merge point "3a1"
(c79ec88). It is a genuine log, but its headline "0 failed" is stale evidence:
two tests it shows green are red on the current tree, and the log carries no
internal provenance (command/date/commit) binding it to any tree state.

## 2. What actually works (verified)
— `grep -o '^test result: ok\. [0-9]* passed' | awk '{s+=$4}'` -> sum passed: 1362;
`grep -c '^test result:'` -> 107; `grep -c '^running [0-9]* tests'` -> 93.
All 107 result lines read "ok", 0 failed (grep for `FAILED|panicked` over the
whole file: zero hits).

## 3. Findings
F-001 [S2]
Where: docs/gates/logs/merge-3a1-c79ec88.test.log:560-566
Code:
```
running 4 tests
test final_frame_is_current ... ok
test ledger_matches_the_artifacts ... ok
test regen ... ignored, rewrites the committed evidence artifacts
test committed_transcripts_match_fresh_runs ... ok

test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 26.66s
```
What is wrong: The same two tests FAIL on the current tree —
audit/runs/baseline/cargo-test-workspace.txt:2764 `test committed_transcripts_match_fresh_runs ... FAILED`
and :2767 `test final_frame_is_current ... FAILED` (panics at
tests/integration/first-vertical/tests/evidence.rs:47:9 and :162:5).
Why it matters: these are the tests that pin committed evidence artifacts to
fresh decoder runs; them failing today means the repo's committed evidence has
drifted from the code, and this log documents a state that no longer
reproduces. Using it as a current "all green" gate is false.
Confidence: HIGH — Verification: read + compared to orchestrator baseline.

F-002 [S3]
Where: docs/gates/logs/merge-3a1-c79ec88.test.log:1-9
Code:
```
   Compiling plugin-api v0.1.0 (/home/z/my-project/crates/plugin-api)
   Compiling template v0.1.0 (/home/z/my-project/crates/_template)
   Compiling input v0.1.0 (/home/z/my-project/crates/input)
```
What is wrong: no header line records the cargo command, commit hash, or date;
the only provenance is the filename (`merge-3a1-c79ec88`). Evidence logs that
cannot self-identify cannot be bound to a tree state.
Confidence: HIGH — Verification: read.

F-003 [S3]
Where: whole file vs docs/gates/phase3-exit.md:50,70
Code (phase3-exit.md:70): `1939 passed / 0 failed / 36 ignored across 163 test`
What is wrong: three different all-green totals coexist for three runs:
1362/0/4 (this log), 1939/0/36 (phase3-exit), 4744/2/51 (current baseline).
The counts are not reconciled anywhere; each doc cites its own log with no
cross-reference explaining the deltas.
Confidence: HIGH — Verification: read + grep.
