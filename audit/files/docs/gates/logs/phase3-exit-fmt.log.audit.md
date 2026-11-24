# AUDIT: docs/gates/logs/phase3-exit-fmt.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A05 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines | 1 (`FMT_RECHECK_EXIT:0`) |
| File verdict | REAL-UNPROVEN (exit marker only; no transcript — the strongest claim a 1-line log can make is unfalsifiable by content) |
| Reachable from shipped entry point? | N/A — Phase-3 gate evidence log, cited by docs/gates/phase3-exit.md:52,212 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sibling log cross-read (:403 `FMT_EXIT_CODE:1`); citing doc quote check; baseline fmt status (AGENT-SPEC §13: FAILED exit 1) |

## 1. One-paragraph verdict
One token of evidence: the entire log is `FMT_RECHECK_EXIT:0`. The pairing is coherent — the sibling 403-line initial-drift log (audited REAL by W17-A10) embeds `FMT_EXIT_CODE:1` with the full drift diff, the citing doc says the drift was fixed on the branch (commit 56f135a) and the re-check was clean — but this log itself carries no command, date, worktree, or output, so its content alone cannot distinguish a run from a typed line. On the current tree the claim is stale anyway: cargo fmt --check FAILS (exit 1) per the orchestrator baseline.

## 2. What actually works (verified)
— verified pairing: docs/gates/logs/phase3-exit-fmt-initial-drift.log:403 `FMT_EXIT_CODE:1` (drift, full diff, REAL per W17-A10) vs this log `FMT_RECHECK_EXIT:0` — the fix-then-recheck narrative is internally consistent, and both exit markers are embedded rather than living only in the citing doc.
— verified citation: phase3-exit.md:52 `**CLEAN (exit 0)** — drift fixed on this branch (commit 56f135a), re-check clean.` and :212 `— post-fix re-check (clean).` — the doc's claim matches the log's single token exactly.
— verified era: 32-file/15-line drift recounted EXACT by W17-A10's sibling audit; the claim "clean post-fix" was era-true.

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase3-exit-fmt.log:1 (whole file)
Code: `FMT_RECHECK_EXIT:0`
What is wrong: content-free evidence — no command line, no timestamp, no worktree, no check output; a single exit token is indistinguishable from a hand-typed line. Contrast the sibling log, which carries the full diff. Its evidentiary weight rests entirely on the doc's narrative and the sibling's authenticity.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: docs/gates/logs/phase3-exit-fmt.log:1 (vs current tree)
Code: `FMT_RECHECK_EXIT:0`
What is wrong: stale relative to today — the orchestrator baseline records `cargo fmt --check` FAILED (exit 1) on the current tree (AGENT-SPEC §13; full_gate fmt section red per W17-A05's 99_run_all audit). The phase3 "clean" state did not survive to HEAD.
Confidence: HIGH — Verification: baseline citation (spec §13).

## 4. Test honesty
Not a test; the underlying command (cargo fmt --check) is not re-runnable here (no cargo on sandbox PATH — cargo-build-budget). UNVERIFIED by execution; era-consistency verified via sibling + doc.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CLEAN (exit 0) — drift fixed on this branch (commit 56f135a), re-check clean" | phase3-exit.md:52 | REAL-at-the-era / STALE-now | this log's token + sibling :403 FMT_EXIT_CODE:1; current tree fmt FAILED (baseline) |
| "post-fix re-check (clean)" | phase3-exit.md:212 | Same as above | same proof |

## 6. Dead / duplicated / unreachable code
N/A (data artifact).

## 7. Dependencies & flags
N/A — captured exit marker of `cargo fmt --all` + `cargo fmt --check` per the citing doc's row 3.

## 8. Open questions for the next agent
- None beyond the standing fmt-drift question: the current-tree fmt failure is already tracked by the full_gate baseline — no new information in this log.
