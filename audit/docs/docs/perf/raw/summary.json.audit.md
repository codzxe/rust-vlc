# CLAIM AUDIT: docs/perf/raw/summary.json (222 lines, machine aggregate)
| Field | Value |
|---|---|
| Auditor | W19-A02 — RECOVERED by W19-R2 |
| Type | JSON, parses clean under python3 json (5 top-level families) |
| Families | decode_ms (6 entries) · peak_rss_kib (7) · probe_ms (3) · seek_ms (10) · startup_ms (4) |
| Verdict | REAL (genuine aggregation of the committed CSVs; every spot-checked cell recomputes exactly) |

## 1. Blunt verdict
This is the `baseline.py summary` recomputation artifact, and it is EXACTLY what it
claims to be: I independently recomputed medians/p95 from the raw CSVs (startup.csv,
probe.csv, decode.csv, seek.csv, memory.csv) with the nearest-rank rule and every one
of the 30 entries I checked matches this file cell-for-cell — including the
`mp4_mjpeg_pcm.mp4 [exit65-refusal]` memory row and the `refusal_response_ms` probe
row (refusals kept as rows, never dropped). It also matches docs/perf/baseline.md's
printed tables cell-for-cell (verified in that ledger this run).

## 2. Claim ledger
| # | Claim (data invariant) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | startup family: help 1.138/1.35, version-warm 0.537/0.605, cold 6.706/7.538, engine-info 0.546/0.694 (n=25/25/5/25) | startup_ms | REAL | Recomputed from raw/startup.csv this run — identical; ns match row counts. |
| 2 | probe family: pcm 1.074/2.347, 2pcm 1.034/1.081, mp4 refusal 0.520/0.548 (n=20 each) | probe_ms | REAL | Recomputed from raw/probe.csv — identical (mp4 rows carry `typed-refusal` outcome in the CSV). |
| 3 | decode family: pcm wall 8/8, decode 7/8, per-frame 0.117/0.133; 2pcm 3/3, 3/3, 0.150 (n=12) | decode_ms | REAL | Recomputed from raw/decode.csv — identical; the CSV's `exit65` mp4 rows are (correctly) NOT aggregated here as decode numbers. |
| 4 | seek family: 8 entries 6.983/6.992/5.804/5.810/5.339/5.351/5.346/5.352 medians (+2 open_response rows) | seek_ms | REAL | Recomputed from raw/seek.csv — all identical; plus `open_response_ms` 1.032/1.023 which the CSV's open_response_ms column supports. |
| 5 | memory family: 7 scenarios incl. `decode-bench:mp4_mjpeg_pcm.mp4 [exit65-refusal]` median 3000/max 3124, session rows 12356/11796/3160 | peak_rss_kib | REAL | Recomputed from raw/memory.csv — identical medians AND maxima; the refusal-path scenario is named as a refusal, not disguised as a decode row. |
| 6 | Cross-doc agreement with docs/perf/baseline.md tables | whole | REAL | baseline.md's §2-§6 tables carry the same 30 numbers (cell-for-cell match — my baseline.md ledger verified them against the CSVs; this file agrees with both). |

## 3. Contradictions
None. All three artifacts (CSVs, this summary, baseline.md) agree; no number appears
in two places with different values.

## 4. Fabrication / inflation indicators
None. The aggregate includes the FAILURE rows (exit65 refusal) as first-class entries
with their own scenario names; min/max/n are carried alongside median/p95 so the
spread is auditable; no target/goal field exists anywhere in the file.
