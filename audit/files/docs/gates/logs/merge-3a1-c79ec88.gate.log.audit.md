# AUDIT: docs/gates/logs/merge-3a1-c79ec88.gate.log
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 18 / 1 gate invocation (10 steps) |
| File verdict | REAL — corroborated green gate run |
| Reachable from shipped entry point? | N/A — committed gate-evidence artifact (produced by `xtask gate`, the repo's own gate tool) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `wc -l` + `rg` over the sibling merge-3a1-c79ec88.test.log (all exit 0): 107 test-result lines, 1362 passed / 0 failed / 4 ignored — corroborates step 4 below. |

## 1. One-paragraph verdict
The best-formed gate evidence in my assignment: the xtask gate summary records command, commit, dirty-flag, date, host, toolchain, per-step timings, and explicit skips with reasons, and its headline "test PASS 1m47s" is corroborated by the committed sibling full test log (1362 passed / 0 failed / 4 ignored, zero FAILED lines). The two defects are the familiar dead pointer (`full log: target/gate-report.json` — not committed) and the absence of raw fmt/clippy transcripts for this merge (only the test sibling exists).

## 2. What actually works (verified)
- Sibling corroboration (executed rg): docs/gates/logs/merge-3a1-c79ec88.test.log — 2028 lines, 107 `test result:` lines, sum = **1362 passed / 0 failed / 4 ignored**, no `FAILED` line anywhere → gate step 4 "test PASS 1m47s" has a real committed backing log.
- Skip honesty: steps 6-9 SKIP "(excluded: --no-ui)" — the flag that caused them is printed in the invocation line (:2 `Running target/debug/xtask gate --no-ui`); step 10 SKIP "(crates/engine-wasm-binding not present)" — a reason, not a silent omission.
- Metadata: commit + dirty flag, ISO timestamp, host, toolchain (rustc 1.99.1 58cf8f9c3) — matches rust-toolchain.toml's pin (channel 1.99.1) exactly.

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/merge-3a1-c79ec88.gate.log:17
Code:
```
full log: target/gate-report.json
```
What is wrong: the machine-readable gate report is pointed at `target/` (build dir, never committed) — the same dead-evidence pointer phase1-exit.md rows 1/5 use. The human summary here IS committed, which is the better half of the pattern, but the full JSON is unrecoverable. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: docs/gates/logs/ (directory)
What is wrong: for this merge only the TEST sibling is committed (merge-3a1-c79ec88.test.log); the fmt/check/clippy/tree steps' raw transcripts are not committed anywhere — steps 1,2,3,5 PASS on this log's say-so alone. (Phase-3 logs commit fmt/clippy transcripts separately, so the pattern exists; it was not applied to this merge.) Confidence: HIGH — Verification: `ls docs/gates/logs/` (dir listing cited in the phase1 ledger).

## 4. Per-function table
N/A (data log). Structure:
| Section | Lines | Content | Verified |
|---|---|---|---|
| invocation+meta | 1-5 | xtask gate --no-ui, c79ec88, 2026-10-21T18:00:45Z | read |
| 10 gate steps | 6-15 | 5 PASS (fmt/check/clippy/test/tree) + 5 SKIP w/ reasons | read; step 4 corroborated by sibling |
| result | 16-17 | SUCCESS 0/10 failed, 5 skipped; JSON pointer | read; pointer dead (F-001) |

## 5. Test honesty
N/A — evidence artifact; internally consistent and cross-corroborated (sibling log totals match the PASS claim).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "result: SUCCESS (0 of 10 failed, 5 skipped)" | log:16 | REAL (for the recorded run) | 5 PASS + 5 SKIP rows above it; test step corroborated by sibling (1362/0/4) |
| "commit: c79ec88 (dirty: no)" | log:4 | UNVERIFIED | git forbidden |
| "10 wasm-pack SKIP (crates/engine-wasm-binding not present)" | log:15 | REAL at that date | that crate does not exist in today's tree either (current wasm work lives in plugin-host-wasm/apps/web) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
xtask gate (repo tool), --no-ui flag, toolchain 1.99.1. No external deps.

## 9. Open questions for next agent
- Which merge waves have gate logs but NO committed test sibling (only this 3a1 pair is complete)? A coverage gap list of merge evidence would quantify the pattern.
- Today's baseline: fmt FAILS on main — find the regression window between 2026-10-21 (this PASS) and now; the phase3 fmt logs (phase3-exit-fmt-initial-drift.log) may bracket it.
