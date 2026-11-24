# AUDIT: docs/gates/logs/phase4-exit-rust-phase4-crates.log
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 166 / n.a. (cargo test output; 10 test-result blocks, ~21 crates compiled) |
| File verdict | REAL (data audit: genuine scoped test transcript, internally consistent, numbers match its citing doc exactly) |
| Reachable from shipped entry point? | n.a. (log artifact). Covers desktop + playlists + lumen-sweep (the then-Phase-4 delta set). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | awk totals -> ok=82 fail=0 ignored=1; grep Running/test-result table (10 binaries); grep FAILED -> 0; sed quote verifies (L21-23, L60-66); rg citation -> phase4-exit.md:55 |

## 1. One-paragraph verdict
A genuine, clean cargo-test transcript for the Phase-4 scoped gate: 82 passed / 0 failed / 1 ignored across 10 test binaries, run from the (now-deleted) worktree `worktrees/p4-gate` against the shared target dir. Its citing doc (docs/gates/phase4-exit.md:55) claims exactly "82 passed / 0 failed across 10 test binaries" with a breakdown (desktop 38, m3u 18, sweep 15, protocol 4, zero_alloc 2, doctests/empty the rest) that reproduces from the log line-by-line — no inflation, no mismatch. The log is old (Phase-4 era; 52.61s compile) and of course proves nothing about today's tree beyond its historical scope, which the citing doc itself states ("Full workspace NOT re-run — deliberate").

## 2. What actually works (verified)
— verified by `awk` over `test result:` lines -> `ok=82 fail=0 ignored=1`; per-binary table extracted by grep: desktop lib 38 (:65), lumen_desktop main 0 (:71), pipeline 4 (:81), protocol 4 (:91), zero_alloc_transport 2 (:99), lumen_sweep 15 (:120), playlists lib 0 (:126), m3u_suite 18 (:150), desktop doctests 1+1 ignored (:158), playlists doctests 0 (:165). Sum = 82. `grep -c FAILED` -> 0.

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase4-exit-rust-phase4-crates.log:1-21 (header lines)
Code:
```
   Compiling lumen-sweep v0.1.0 (/home/z/my-project/worktrees/p4-gate/tools/sweep)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 52.61s
```
What is wrong: The log carries no capture header (worktree/commit/date/exit-code) — provenance is only inferable from the compile paths, and the source worktree (`/home/z/my-project/worktrees/p4-gate`) no longer exists on this machine (`ls` -> cannot access), so the log can never be re-derived or byte-verified against that state. Why it matters: the wave-00 gate-log convention (capture-time headers) was adopted later; this older log predates it and would fail that convention. What correct looks like: retroactive provenance annotation like the phase7 round-1 logs received (per phase7-exit.md:67-74). Confidence: HIGH — Verification: read + ls.

## 4. Per-function table
Not applicable (data/log). Binaries covered: desktop (lib + doctests + pipeline + protocol + zero_alloc_transport), lumen_desktop main, playlists (lib + m3u_suite + doctests), lumen_sweep.

## 5. Test honesty
The log is the evidence itself. Honesty of the citing doc's use: phase4-exit.md:55 states the scoped-run rationale explicitly (workspace covered earlier at 1939/0/36 by the Phase-3 gate) rather than implying a full-suite pass. Cross-check of that older 1939/0/36 figure: UNVERIFIED (Phase-3 log not in this assignment; would be its own data audit).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "82 passed / 0 failed across 10 test binaries — desktop 38/0 ... playlists m3u_suite 18/0, lumen_sweep 15/0, desktop protocol 4/0 + zero_alloc_transport 2/0 + 5 zero/empty mains" | docs/gates/phase4-exit.md:55 | REAL | awk totals + per-binary grep table above; every sub-number reproduces (38, 18, 15, 4, 2, zeros) |

## 7. Dead / duplicated / unreachable code
n.a.

## 8. Dependencies & flags
Debug `test` profile; shared CARGO_TARGET_DIR (`/home/z/my-project/target/debug/deps/...` paths) — consistent with the repo's documented shared-target convention.

## 9. Open questions for the next agent
- Confirm the Phase-3 "1939/0/36" workspace figure against the Phase-3 exit log (out of this assignment's file set).
