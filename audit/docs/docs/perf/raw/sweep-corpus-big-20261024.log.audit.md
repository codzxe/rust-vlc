# CLAIM AUDIT: docs/perf/raw/sweep-corpus-big-20261024.log (696 lines — adversarial sweep log)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 2 (sweep coverage; typed failures) |
| REAL / ... / FALSE | 2 REAL |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Raw SWEEPLINE log: 696 rows over the big adversarial corpus (dirty + media fixtures) with
per-file outcomes ok / dirty-reject / unsupported and TYPED error strings
(demux.avi.no-movi, probe.no-demuxer with sniffed bytes). This is the "88-file adversarial
sweep, 0 panics" evidence family the error-surface audit cites (W4-R2), and it is cited
by docs/worklog/archive-2026-10.md. The log's honest handling: malformed files produce
typed rejects, not successes — matches the D-4 "never fake success" discipline.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 696-row sweep over corpus-big fixtures | :1-696 | REAL | wc -l = 696; rows are tab-structured SWEEPLINE records. |
| 2 | Malformed inputs yield typed rejects (dirty-reject/unsupported with stage+code) | sample rows | REAL | Rows: "dirty-reject demux corrupt demux.avi.no-movi", "unsupported probe unsupported probe.no-demuxer (sniffed ...)". |

## 3-4. Contradictions / fabrication indicators
None; none — data file, error text verbatim.
