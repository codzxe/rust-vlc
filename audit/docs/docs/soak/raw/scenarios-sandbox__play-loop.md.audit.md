# CLAIM AUDIT: docs/soak/raw/scenarios-sandbox__play-loop.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 1 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Soak scenario report whose arithmetic is exact (-16.1 B/s → -0.058 MB/h
recomputes to -0.05796; 482−73=409 samples; warmup 73 ≈ 15% boundary),
whose three named fixtures exist in-tree, and whose independent re-check
(the W24-B soak review, already audited) matched these numbers. The
report self-labels its MB/h figures as EXTRAPOLATIONS, not measurements.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | fixtures: avi_mjpeg_pcm.avi, avi_mjpeg_2pcm.avi, malformed trunc3 | :17 | REAL | all three exist in tests/fixtures/generated/{valid,malformed}/ (ls) |
| 2 | "Live-bytes slope: -16.1 B/s = -0.058 MB/h (EXTRAPOLATION …) — negative" | :20 | REAL | python3: -16.1×3600/1e6 = -0.05796 ✓ rounds to -0.058 |
| 3 | "Samples: 482 total, 73 warmup-excluded, measured window 407.0 s" | :16 | REAL | 482−73=409; 15% of 480 s = 72 → 73-boundary consistent; cross-verified by w24-b review §"re-computed" |
| 4 | "PASS — every scenario stayed inside its configured liveness and memory budgets" | verdict | REAL-UNPROVEN | run record (release build, wt/w24b worktree — not re-runnable here) |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — extrapolation caveat stated three times; error-lane counts kept
(0 error-lane opens/closes recorded, not omitted).

Findings: F-001 [S4] manifest path points at a worktree
(/home/z/my-project/wt/w24b/) — the committed copy is
tests/soak/scenarios-sandbox.toml.
Commands: python3 arithmetic, ls fixtures (exit 0).
