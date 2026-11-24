# AUDIT: docs/gates/logs/wave2-merge.test.log
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 2316 / n.a. (cargo test output; 102 test binaries, 127 `test result:` lines) |
| File verdict | SHALLOW (data audit: genuine cargo-test transcript, but stale — contradicts current tree on 2 tests) |
| Reachable from shipped entry point? | n.a. (log artifact, not code) |
| Findings | S0: 1 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | awk totals -> ok=1526 fail=0 ignored=16; grep -n evidence-test names -> L729/L732; baseline cross-check -> FAILED x2; sed -n '725,735p' -> quote verified |

## 1. One-paragraph verdict
This is a real cargo-test transcript (correct libtest formatting, per-binary hashes, timings, 24 Doc-tests sections, doctest merge footer). It claims a fully green run: 1526 passed / 0 failed / 16 ignored across 102 test binaries. But the current tree FAILS two of the tests this log shows passing (`committed_transcripts_match_fresh_runs`, `final_frame_is_current`), and the test binary hashes differ from the baseline run — this log was captured against a different source state and is not reproducible as "passing" today. Coverage is also only ~32% of the workspace suite (1526 vs baseline 4744 passed).

## 2. What actually works (verified)
— verified by `awk` over `test result:` lines -> `ok=1526 fail=0 ignored=16` (internally consistent: 16 `... ignored` lines found at L270,271,293,294,316,317,352,579,580,616,731,1137,1220,2126,2127,2133). Log format is genuine libtest output, not hand-typed.

## 3. Findings
F-001 [S0]
Where: docs/gates/logs/wave2-merge.test.log:725-733
Code:
```
     Running tests/evidence.rs (target/debug/deps/evidence-a80d856a24b49d2c)

running 4 tests
test final_frame_is_current ... ok
test ledger_matches_the_artifacts ... ok
test regen ... ignored, rewrites the committed evidence artifacts
test committed_transcripts_match_fresh_runs ... ok
```
What is wrong: This log asserts both first-vertical evidence tests pass. The orchestrator baseline on the CURRENT tree shows both FAILED: audit/runs/baseline/cargo-test-workspace.txt:2764 `test committed_transcripts_match_fresh_runs ... FAILED`, :2767 `test final_frame_is_current ... FAILED`, panicking at `tests/integration/first-vertical/tests/evidence.rs:47:9` ("the committed transcript for `open` drifted from a fresh run") and `:162:5`. The baseline binary is `evidence-41b28bd4ca5aee5a` (baseline L2761) vs this log's `evidence-a80d856a24b49d2c` — different source state. Why it matters: any gate doc that cites this log as proof that tests pass (worklog archive-2026-10.md:7432 does: "Gate result: PASS ... Raw log committed: docs/gates/logs/wave2-merge.test.log") is citing non-reproducible, stale evidence; the two tests fail today. What correct looks like: regenerate the log on the current tree (it would show 2 failures) or stop citing it as a pass. Confidence: HIGH — Verification: read + cross-checked baseline (read).

F-002 [S2]
Where: docs/gates/logs/wave2-merge.test.log (whole file; totals)
Code:
```
test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.47s
```
What is wrong: Aggregate coverage is 1526 ok / 16 ignored over 102 binaries — 32% of the current workspace suite (baseline: ok=4744 fail=2 ignored=51). The log is a wave2-era partial-suite gate, not a full-workspace run. Why it matters: readers of gate docs may take "tests PASS, raw log attached" as full-suite evidence. What correct looks like: label the log with the package set it covers. Confidence: HIGH — Verification: ran (awk totals both files).

F-003 [S3]
Where: docs/gates/logs/wave2-merge.test.log:270-271, 293-294, 352, 1137, 1220, 2126-2133
Code:
```
test mjpeg_decode_throughput_contract_path ... ignored, ED-FIX-1 M13 throughput measurement: cargo test -p codecs --release --ignored (numbers in the worklog ED-CORPUS-1 AFTER-report)
test rebrand_skeleton::about_model_renders_nova_logo ... ignored, runtime plane does not exist yet (Phase 2+): no About model or engineInfo surface — doc 07 §8.3 step 4
```
What is wrong: 16 tests are skipped with reasons pointing at external worklogs ("numbers in the worklog ED-CORPUS-1 AFTER-report") that are not in this log; three rebrand_skeleton tests are ignored because the "runtime plane does not exist yet" — i.e., features the log's era claimed as future work. Why it matters: "0 failed" hides 16 not-run tests, some of which are the only coverage of throughput/rebrand claims. What correct looks like: gate docs must state ignored != passed. Confidence: HIGH — Verification: read.

## 4. Per-function table
Not applicable (data/log file, no functions). Sections verified: unittests (L2-…), integration tests (tests/*.rs binaries L715-…), doctests (L2262-2316, 24 Doc-tests headers, final "all doctests ran in 0.19s").

## 5. Test honesty
Not applicable (this file IS test output; honesty judged in F-001/F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Gate result: PASS ... tests 1526 passed / 0 failed / 16 ignored ... Raw log committed: docs/gates/logs/wave2-merge.test.log" | docs/worklog/archive-2026-10.md:7432 | SHALLOW | log totals match (1526/0/16), but the pass is stale vs baseline (2 FAILED now) — see F-001 |

## 7. Dead / duplicated / unreachable code
n.a.

## 8. Dependencies & flags
n.a. (log); binaries run under debug `test` profile per header L1.

## 9. Open questions for the next agent
- When (commit/date) was this log captured? Not recorded in the log itself — no timestamps beyond per-test durations. UNVERIFIED (no provenance header).
- Does phase7-exit.md cite this log? Checked: no direct citation found in docs/gates/phase7-exit.md (rg 'wave2-merge' -> no hits outside the worklog).
