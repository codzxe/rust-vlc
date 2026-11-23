# AUDIT: crates/playlists/tests/fixtures/hls/ts_vod/pl0.ts (binary fixture)
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Size / type | 8 084 bytes; `file` → "MPEG transport stream data" (verified) |
| File verdict | REAL artifact, INERT bytes (no test reads its content) |
| Cited as evidence by | docs/feature-matrix.toml:1167 (row for m3u.rs, "3 ffmpeg-generated fixtures byte-identical ... 38/0"); docs/formats/streaming-hls.md:99-100; docs/worklog/archive-2026-10.md:9461 |
| Findings | S4: 1 |

## 1. Blunt verdict
Genuine MPEG-TS: sync byte 0x47 at offset 0, PAT/PMT structure, "FFmpeg" registration descriptor visible at offset 26 (`od` dump: `48 12 01 06 46 46 6d 70 65 67` = FFmpeg). Size 8 084 exactly matches the first segment pin in both `ts_vod/pl.m3u8` (EXTINF pl0.ts) and the `single_file` fixture's first BYTERANGE `8084@0`. No test reads its bytes: hls_attrs.rs only COUNTS `.ts` files in the directory (read_dir, L77-83) — existence is load-bearing for `entries == .ts files on disk`, content is not. The streaming crate's e2e uses a DIFFERENT tree (`tests/fixtures/hls/ts_vod/seg000.ts` at workspace root via testkit). Worklog's "101 420 B total" for the three fixture families verified by summation (35 582 + 35 712 + 30 126 = 101 420 exactly).
F-001 [S4]: bytes never read by any test — inert realism; fine for a parse-only card (5.O1 parses playlists, not segments), but a matrix reader could over-read "fixtures byte-identical" as segment-content verification. Confidence HIGH — Verification: od dump, ls, rg for readers.
