# CLAIM AUDIT: docs/perf/raw/parity-v4-durations-20261024.tsv (21 lines — raw durations)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 (per-fixture duration responses) |
| REAL / ... / FALSE | 1 REAL |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Raw TSV: fixture path + verbatim JSON of a duration query response
(requestId/session/gen/result.completed.duration). 20 rows over generated AVI fixtures
(dirty + media). Consistent with the engine's typed event JSON shape. No doc found citing
this file by name (grep: 0 hits outside docs/perf) — data without a derived claim; no
contradiction possible.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Duration-query transcripts per fixture | :2-21 | REAL | Rows parse; JSON shape matches the command layer's completed/duration envelope. |

## 3-4. Contradictions / fabrication indicators
None; none — data file.
