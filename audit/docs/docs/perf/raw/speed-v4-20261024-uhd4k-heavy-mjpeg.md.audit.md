# DATA AUDIT: docs/perf/raw/speed-v4-20261024-uhd4k-heavy-mjpeg.md (credit: W19-A06, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | 2-row summary (ffmpeg + lumen) over 2 heavy UHD4K MJPEG AVIs, best-of-2 |
| Sibling raw | speed-v4-20261024-uhd4k-heavy-mjpeg.json (per-file, repeat=2) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## Verdict
Everything EXCEPT the p95 column derives exactly from the sibling JSON (python3
recompute): med walls = medians of `wall_best` (ffmpeg 1.2199/3.7643 -> 2492.1 ms ✓;
lumen 1.7790/12.8747 -> 7326.8 ms ✓, honoring the "best-of-2" label); med fps 232.2/
89.3 ✓; med realtime 1.65x/0.84x ✓; ratio 2492.1/7326.8 = 0.34x ✓ (lumen ~3x slower on
the heavy lane — honestly published, including 0127's 0.43x realtime = sub-realtime).
S3 (PROVEN, not speculative): the p95 column is broken — "p95 wall 1219.9 ms" vs its own
median 2492.1 ms (ffmpeg) and "1779.0 ms" vs 7326.8 ms (lumen). A 95th percentile cannot
sit below the median of the same set: the two per-file bests are {1.2199, 3.7643}
(ffmpeg) and {1.7790, 12.8747} (lumen); the printed p95s are exactly the LOWER sample of
each pair (1.21987 -> 1219.9; 1.77898 -> 1779.0), i.e. the percentile function returns
the MIN for n=2. Same defect class as speed-baseline-20261022.md (4 rows there), here
provable because the raw JSON is committed. Run not re-runnable (corpus
/tmp/corpus-7d-speed off-repo); the file is uncited by name in docs/ (orphan raw pair).
Quotes byte-verbatim from Read output.
