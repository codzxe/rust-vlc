# CLAIM AUDIT: docs/perf/raw/mjpeg/speed-before-avi-av.json (210 lines, machine artifact)
| Field | Value |
|---|---|
| Auditor | W19-A07 — RECOVERED by W19-R2 |
| Type | JSON, parses clean (corpus /tmp/corpus-w7b / repeat=1 / runners [ffmpeg,lumen] / per_file ×8, all ok) |
| Rows | 8 avi-av lanes, both runners measured per row (wall_best/median/cpu_best/fps/realtime_x) |
| Verdict | REAL (genuine speed_bench artifact, wave-7b "before" baseline; companion .md reconciles) |

## 1. Blunt verdict
Machine artifact and its rendered companion agree: I recomputed the .md's table from
this JSON — medians (ffmpeg 244.3 ms / lumen 182.9 ms), med fps (286.1 / 349.1), med
realtime (21.73× / 26.16×) and the headline ratio **1.34×** all reproduce exactly.
The one wrinkle is a p95 convention: the .md's "p95 wall" cells are the 7th-smallest of
8 (truncated rank) for BOTH runners — ffmpeg 627.5 (not the 1589.1 worst) and lumen
668.3 (not the 3791.6 worst) — consistent across rows, so it is a rank-rule nit, not
outlier-hiding (it demotes ffmpeg's own worst too).

## 2. Claim ledger
| # | Claim (data invariant) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 8 avi-av rows, best-of-1 (repeat=1), both runners ok on every row | per_file | REAL | Parsed: 8 rows, `repeat: 1`, all `ok:true` both sides; fields rel/class/bytes/frames/dur_s present per row. |
| 2 | .md companion row: ffmpeg med 244.3 ms / p95 627.5 / fps 286.1 / realtime 21.73× | speed-before-avi-av.md | REAL | Recomputed from JSON: med(ffmpeg walls)=244.3 ms ✓; med fps 286.1 ✓; med realtime 21.73 ✓; 627.5 = 7th-smallest (see F-001). |
| 3 | .md companion row: lumen med 182.9 ms / p95 668.3 / fps 349.1 / realtime 26.16× / ratio 1.34× | speed-before-avi-av.md | REAL | Recomputed: med(lumen walls)=182.9 ms ✓; fps 349.1 ✓; realtime 26.16 ✓; ratio med(ff)/med(lum)=244.3/182.9=1.34 ✓ — the X1b-era "before" parity figure. |
| 4 | Consumer: cited as raw evidence by docs/perf/mjpeg-simd-20261024.md | :90 of that doc | REAL | The SIMD-ablation doc names `speed-before-avi-av.{json,md}` among its raw artifacts (rg-verified). |

## 3. Contradictions
None between JSON and .md medians. F-001 [S4]: the .md's "p95 wall" uses the
7th-smallest-of-8 rank (int-truncation) rather than the nearest-rank 8th its header
implies — affects both runners symmetrically (ffmpeg 627.5 vs true-worst 1589.1; lumen
668.3 vs 3791.6), so no runner is favored; the repo's other perf docs (baseline.md) use
the ceil() rank that DOES hit the worst sample.

## 4. Fabrication / inflation indicators
None. The worst row (lumen 3.792 s on media/0035 — 0.48× ffmpeg on that lane) is fully
present in the JSON at full precision; only the .md's p95 column blunts it via the rank
convention, and the medians that the consumer docs actually quote are exact.
