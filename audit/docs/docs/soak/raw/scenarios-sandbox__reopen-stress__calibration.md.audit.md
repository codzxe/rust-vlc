# CLAIM AUDIT: docs/soak/raw/scenarios-sandbox__reopen-stress__calibration.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 1 |
| REAL 1 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | Calibration run of reopen-stress: 1500 cycles, 3.0 s, PASS — the sizing probe for the full 150k run | REAL | same schema as the full run; 16500 frames = 11 x 1500 cycles — scale-consistent with the 150k sibling (1.65M frames); captured 18:35, ~40 min before the full 19:14 run — a real calibration-then-scale sequence |

## 1. Blunt verdict
The calibration twin of the full soak run — identical report shape at 1% of the cycles, numbers exactly proportional (16500 vs 1650000 frames). The pair (calibration + full run) is how honest soak methodology looks.
## 3. Contradictions: none. ## 4. Indicators: none.
