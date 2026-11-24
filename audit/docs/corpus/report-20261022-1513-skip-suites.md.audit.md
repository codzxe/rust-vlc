# CLAIM AUDIT: docs/corpus/report-20261022-1513-skip-suites.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 5 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 / 1 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Honest corpus report whose central claim is that it is NOT a fresh measurement
("--skip-suites ... persisted results ... re-aggregated ... NOT a fresh measurement") —
the abstention is stated four times. It surfaces a real defect (garbage rejection 10/11,
"must be 100%") instead of hiding it.

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "mode: --skip-suites ... NOT a fresh measurement ... aggregation validation" | REAL | verbatim L5-6 (sed-verified); suite rows marked reused |
| 2 | "garbage rejection 10/11 (must be 100%)" | REAL | honest defect disclosure — a failing metric is printed, not buried |
| 3 | "supported clean AVIs walked to EOS 85/85; dirty AVIs typed terminal 35/35 (no panic, no hang)" | REAL | persisted rows carried with the caveat from claim 1 — data provenance is the last full run |
| 4 | "corpus seed 6033317083842721047527680438 ... replay with gen_corpus.py --replay" | REAL-UNPROVEN | reproducibility path exists in repo (gen_corpus.py); not re-run (corpus generation is heavy) |
| 5 | "probe latency p50/p95/max 409µs/684µs/1210µs" | REAL (persisted) | same reuse caveat — the report never claims it measured them now |

## 3-5. Contradictions / fabrication / matrix rows
None / none — over-claiming is structurally prevented by the mode line. Commands: head -60.
