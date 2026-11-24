# CLAIM AUDIT: docs/perf/raw/startup.csv (81 lines — raw measurement data)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 (real startup-latency capture) |
| REAL / ... / FALSE | 1 REAL |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Genuine CSV measurement capture: header mode,variant,run,wall_ms,status + 80 data rows
(--help/--version warm/cold runs with per-run wall times ~1.2-1.4 ms warm). Cited as input
by tools/perf/baseline.py (the perf tooling). No prose claims inside the file; nothing to
contradict. Values are plausible micro-benchmarks of a small CLI.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Startup latency raw data (mode x variant x run) | rows 2-81 | REAL | rg -l -> tools/perf/baseline.py consumes it; 80 rows parse as CSV. |

## 3-4. Contradictions / fabrication indicators
None; none — data file, no claims.
