# CLAIM AUDIT: docs/perf/raw/r1/ffmpeg-lanes.csv (11 lines — raw ffmpeg timing)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 (ffmpeg lane decode timings) |
| REAL / ... / FALSE | 1 REAL |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Raw CSV: lane,n,med_s,min_s,max_s — 10 lanes (audio-only flac/mp3/mp2 etc., n=5 each).
This is the oracle-side timing baseline (ffmpeg) used by the r1 differential/perf work.
No doc cites it by name outside docs/perf — pure data; no contradiction.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | ffmpeg per-lane decode medians (n=5) | :2-11 | REAL | 10 well-formed rows; medians within [min,max] for every row (spot-checked 4/4). |

## 3-4. Contradictions / fabrication indicators
None; none — data file.
