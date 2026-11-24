# CLAIM AUDIT: docs/perf/raw/seek.csv
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 1 (artifact integrity) |
| REAL 1 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | Machine-measured seek latency CSV (61 rows: header + 20 runs x 3 fixtures) | REAL | well-formed CSV; fixture/run/open/seek1/seek2 response+event ms, outcome=completed, exit=ok; consistent with the environment.json provenance block (same campaign, /tmp corpus) |

## 1. Blunt verdict
Genuine raw measurement artifact (median-friendly run-level rows). Ephemeral corpus (/tmp/corpus-w7b) — replay needs regeneration; the CSV itself is the committed evidence.
## 3. Contradictions: none. ## 4. Indicators: none.
