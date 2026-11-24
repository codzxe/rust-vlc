# DATA AUDIT: docs/perf/raw/mjpeg/speed-before-avi-video.md (credit: W19-A08, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | 2-row summary over 8 avi-video files, best-of-1; lumen row n=5 |
| Sibling raw | speed-before-avi-video.json (8 per-file rows; 3 lumen DNF rc=65) |
| Cited by | docs/perf/mjpeg-simd-20261024.md:90 ("Pre-SIMD end-to-end raw") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## Verdict
All summary numbers derive EXACTLY from the JSON (python3 recompute): ffmpeg med wall
447.7 ms ✓, med fps 166.6 ✓, med realtime 9.58x ✓; lumen (5 ok files) med wall 1507.8 ms
✓, med fps 76.9 ✓, med realtime 1.39x ✓; ratio 447.7/1507.8 = 0.297 -> "0.30x" ✓ —
lumen ~3.3x slower, honestly published. The lumen row's n=5 (vs files: 8) quietly
discloses that 3 of 8 files are excluded — but the md never says WHY: the JSON shows
`"ok": false, "dnf": true, "rc": 65` for media/0011, 0019, 0027 (lumen cannot decode
them; rc 65 = the no-decoder exit seen in the baseline HEVC smoke). S4: exclusion cause
missing from the md.
S3 (proven, family-wide): the "p95 wall" column is not a p95. Recomputed: ffmpeg's
"2021.0 ms" is the 7th-of-8 sorted value (= the 87.5th percentile) and lumen's "1983.2 ms"
is the 4th-of-5 (= the 80th percentile) — consistent with the summarizer formula
sorted[int(0.95*n)-1], which for n=2 returns the MINIMUM (proved in my
speed-v4-20261024-uhd4k-heavy-mjpeg.md audit, where p95 < median). The label overstates
tail coverage at every n in this md family.
Corpus /tmp/corpus-w7b off-repo; run not re-executable. Quotes byte-verbatim from Read.
