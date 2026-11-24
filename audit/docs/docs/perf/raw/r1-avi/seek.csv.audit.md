# CLAIM AUDIT: docs/perf/raw/r1-avi/seek.csv
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 1 |
| REAL 1 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | r1 AVI seek-latency CSV (21 rows; corpus file 0052-avi-video.avi, ~100ms seeks) | REAL | same schema as parent seek.csv; 5 runs x fixtures; values self-consistent (event >= response ms each row — physical ordering holds) |

## 1. Blunt verdict
Genuine r1-campaign raw artifact; note it honestly shows SLOWER seeks (~100ms vs first-vertical's ~7ms) on the corpus AVI — recorded as-is, no cherry-picking visible.
## 3. Contradictions: none. ## 4. Indicators: none.
