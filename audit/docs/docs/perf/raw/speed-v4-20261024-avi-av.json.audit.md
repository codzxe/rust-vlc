# CLAIM AUDIT: docs/perf/raw/speed-v4-20261024-avi-av.json (210 lines, machine artifact)
| Field | Value |
|---|---|
| Auditor | W19-A09 — RECOVERED by W19-R2 |
| Type | JSON, parses clean (corpus /tmp/corpus-w7b / repeat=2 / runners [ffmpeg,lumen] / per_file ×8, all ok) |
| Rows | 8 avi-av lanes (best-of-2), both runners measured per row |
| Verdict | REAL (genuine speed_bench artifact — the corpus-v4 X1b-era "after" run) |

## 1. Blunt verdict
Machine artifact; its rendered companion's every cell I recomputed is exact —
medians (ffmpeg 81.7 ms / lumen 73.6 ms), fps (553.7 / 619.8), realtime (69.45× /
50.65×), ratio **1.11×**. Compared against its "before" sibling (speed-before-avi-av,
best-of-1), the same corpus shows the X1b-era median improvement (182.9 → 73.6 ms) with
the worst lane 3.792 → 1.974 s — both runs committed, neither dropped.

## 2. Claim ledger
| # | Claim (data invariant) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 8 avi-av rows, best-of-2 (repeat=2), both runners ok on all rows | per_file | REAL | Parsed: `repeat: 2`, 8 rows, `n: 2` per runner entry, all ok. |
| 2 | .md companion: ffmpeg med 81.7 ms / p95 219.2 / fps 553.7 / realtime 69.45× | speed-v4-20261024-avi-av.md | REAL | Recomputed: med(ffmpeg)=81.7 ms ✓, med fps 553.7 ✓, med rt 69.45 ✓; 219.2 = 7th-smallest (rank convention, see F-001). |
| 3 | .md companion: lumen med 73.6 ms / p95 482.0 / fps 619.8 / realtime 50.65× / ratio 1.11× | same | REAL | Recomputed: med(lumen)=73.6 ✓, fps 619.8 ✓, rt 50.65 ✓; ratio 81.7/73.6 = 1.11 ✓. |
| 4 | Before/after continuity with speed-before-avi-av.json (same corpus /tmp/corpus-w7b) | both files | REAL | Same 8 rel lanes in both JSONs (0021/0022/0035/…); medians 182.9→73.6 ms and worst lane 3.792→1.974 s — the improvement story is derivable from committed pairs. |

## 3. Contradictions
None. F-001 [S4] (family-wide): the .md companion's "p95 wall" cells are the
7th-smallest of 8 (int-truncated rank) instead of the nearest-rank 8th — symmetric for
both runners (ffmpeg 219.2 vs worst 587.9; lumen 482.0 vs worst 1974.0), the same
convention as speed-before-avi-av.md; medians the consumer docs quote are unaffected.

## 4. Fabrication / inflation indicators
None. The worst rows (lumen 1.974 s on media/0035 — slower than ffmpeg's 0.588 s on
that lane) are preserved at full precision in the JSON.
