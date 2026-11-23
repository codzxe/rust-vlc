# AUDIT: crates/playlists/tests/fixtures/hls/ts_vod/pl2.ts (binary fixture)
| Field | Value |
|---|---|
| Auditor | W10-A10 | Date | 2026-11-23 |
| Type / size | `file` -> "MPEG transport stream data"; 7708 bytes = exactly 41 × 188-byte TS packets (7708 % 188 == 0), 0x47 sync verified at packet boundaries via od |
| File verdict | REAL fixture — real ffmpeg-captured TS segment, consumed by byte-exact e2e |
| Reachable from shipped entry point? | Fixture is test-only; the lane it feeds (streaming hls VOD e2e) is test-only (HLS sessions unwired, see master.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `od -A d -t x1` (sync/PAT/PMT/PES verified: first packet `47 40 11 12 …` PAT, `47 40 00 12` PES, PMT descriptor contains "FFmpeg"+"Service01"), `md5sum` (0b0a57a9…), `cmp` vs pl3.ts (differ — consecutive segments, continuity counter 0x12 vs 0x13) |

## 1. Blunt verdict
Genuine MPEG-TS media (PAT + PMT + PES with ffmpeg's service descriptors), 41 aligned packets, a real consecutive segment of the ts_vod lane (pl3.ts is its successor with advanced continuity counters). Not synthetic padding.
## 2. Consumers & citations
- crates/streaming/tests/hls_vod_e2e.rs:165-193 — `serve_lane("ts_vod", "ts")` serves it over a real mock socket; `on_disk.extend_from_slice(&lane_bytes("ts_vod", …))` proves the session's served bytes are byte-exact against the fixture.
- crates/streaming/tests/netfuzz_sweep.rs; sibling pl.m3u8 lists pl0..pl4 with real ffmpeg EXTINF durations (sum 9.046544 s).
- docs/feature-matrix.toml:1167 (playlist-m3u row) cites "3 ffmpeg-generated fixtures byte-identical" for the lane family — accurate as to these files' provenance.
## 3. Verdict
REAL. No fabrication indicators: real capture artifacts (continuity counters advance across pl2→pl3, stuffing bytes, real PMT descriptors), correct 188-byte alignment, honest provenance chain to ffmpeg.
