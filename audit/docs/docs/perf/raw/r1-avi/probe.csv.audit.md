# DATA AUDIT: docs/perf/raw/r1-avi/probe.csv (credit: W19-A09, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Rows | 30 data rows (3 fixtures x 10 runs), header `fixture,run,write_ms,response_ms,media_info_ms,outcome,exit` |
| Cited by | docs/perf/breadth.md:215 — "r1-avi/probe.csv (30 rows)" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Genuine-looking timing data that cross-derives EXACTLY to the doc table that cites it:
- breadth.md:121 "responses 2.45-2.47 ms" == my medians of response_ms: 0052=2.4675,
  0073=2.466, 0124=2.454 (python3 statistics.median over the 30 rows).
- breadth.md:121 "2.48-2.49 ms" open->media.info-ready == media_info_ms medians:
  0052=2.492, 0073=2.4865, 0124=2.4815.
- Internal arithmetic: write_ms + response_ms ≈ media_info_ms per row (e.g. row 2:
  0.029 + 12.669 = 12.698 vs 12.711 reported) — consistent.
- First run of each fixture is 5-12x slower (cold start), retained not smoothed — the
  signature of a real measurement, not a fabricated uniform table.
- Row count claim "30 rows" is exact (31 lines incl. header).
S4 (only defect): zero provenance — no tool, binary version, or command line in the file;
fixtures 0052/0073/0124 AVI corpus files are not in the repo, so a re-run is impossible
(UNVERIFIED: no fixture, media off-repo). Quotes byte-verbatim: `sed -n '1,2p;12p;22p'`
spot-check against Read output. "The CSVs are the truth; the tables above are their
medians" (breadth.md:218-219) — verified true for this CSV.
