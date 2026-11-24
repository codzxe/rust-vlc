# DATA AUDIT: docs/perf/raw/mjpeg/speed-before-avi-av.md (credit: W19-A02, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | 2-row class summary (ffmpeg + lumen) over 8 avi-av files, best-of-1 |
| Cited by | docs/perf/mjpeg-simd-20261024.md:90 ("Pre-SIMD end-to-end raw") |
| Sibling raw | docs/perf/raw/mjpeg/speed-before-avi-av.json (8 per-file rows) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
EVERY number in this summary derives EXACTLY from the sibling JSON's per-file data
(python3 recompute, output above): ffmpeg med wall 244.3 ms ✓, p95 627.5 ms ✓, med fps
286.1 ✓, med realtime 21.73x ✓; lumen 182.9 ms ✓, 668.3 ms ✓, 349.1 ✓, 26.16x ✓; ratio
244.3/182.9 = 1.335 -> "1.34x" ✓. The md is a faithful render of the raw, not a re-assert.
Honesty markers: "best-of-1" is disclosed up front; the raw JSON keeps per-file n=1 walls
including the case where lumen LOSES badly (media/0035: lumen 3.79 s vs ffmpeg 1.59 s,
0.89x realtime — 2.4x slower), so the favorable class median is not cherry-picked from a
hidden failure; runner=ffmpeg AND lumen both present, so unlike speed-baseline-20261022.md
this file is a real two-sided parity measurement. Corpus /tmp/corpus-w7b is off-repo so
no re-run (UNVERIFIED run). S4: the p95 rule (7th-of-8 nearest-rank) is undocumented.
CORRECTION (self-review): the ratio convention is CONSISTENT across the perf docs —
ratio = ffmpeg_wall/lumen_wall (1.34x here, lumen faster; 0.47-0.69x in KNOWN-LIMITS/
breadth with lumen slower; 0.34x in the uhd4k file). Quotes byte-verbatim from Read output.
