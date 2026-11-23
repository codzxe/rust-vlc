# AUDIT: crates/playlists/tests/fixtures/hls/ts_vod/pl4.ts (binary fixture)
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Size / type | 4 136 bytes; `file` → "MPEG transport stream data" (verified) |
| File verdict | REAL artifact, INERT bytes (no test reads its content) |
| Cited as evidence by | docs/feature-matrix.toml:1167; docs/formats/streaming-hls.md:99-100; docs/worklog/archive-2026-10.md:9461 |
| Findings | S4: 1 |

## 1. Blunt verdict
Same genuine-ffmpeg MPEG-TS shape as pl0.ts (0x47 sync, PAT/PMT, FFmpeg descriptor in the `od` dump). Size 4 136 matches the LAST segment of `ts_vod/pl.m3u8` and the final BYTERANGE pin `4136@31208` of the `single_file` fixture — and 8084+7708+7708+7708+4136 = 35 344 = single_file/pl.ts's actual on-disk size, so the two fixture families are byte-budget-consistent with each other. Like pl0.ts, its content is never read by any test (hls_attrs counts directory entries only).
F-001 [S4]: bytes never read by any test — inert realism (same as pl0.ts and, per W10-A07's audits, the fmp4/*.m4s siblings); the count assertion needs the file to EXIST, nothing needs it to contain TS. Confidence HIGH — Verification: od dump, ls, summation check, rg for readers.
