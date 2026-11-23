# AUDIT: crates/playlists/tests/fixtures/hls/ts_vod/pl1.ts (binary fixture)
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Size / type | 7708 bytes (= 41 × 188-byte TS packets exactly); `file` → "MPEG transport stream data" |
| File verdict | REAL (genuine FFmpeg-encoded TS media segment, loaded and demuxed by real tests) |
| Reachable from shipped entry point? | N/A (test asset) — served over a REAL loopback socket by crates/streaming/tests/hls_vod_e2e.rs:59-66 `serve_lane("ts_vod", "ts")` (route `/hls/ts_vod/pl1.ts`) and drained through the real TS demuxer; also on disk for crates/playlists/tests/hls_attrs.rs:77 (read_dir segment count, ffprobe truth 9.046543) and the netfuzz_sweep.rs:127 corpus. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Verdict
Genuine encoder output, not hand-crafted bytes: the PAT/PMT carries the FFmpeg service descriptor (`od` offset 0x1a: `46 46 6d 70 65 67` "FFmpeg", "Service01") — matching the matrix evidence claim "3 ffmpeg-generated fixtures byte-identical, EXTINF sums vs ffprobe 9.046543" (docs/feature-matrix.toml:1168). Referenced as segment 2 of 5 by its sibling `pl.m3u8` (`#EXTINF:1.996922,\npl1.ts`). Its bytes are actually fetched and demuxed in the hls_vod_e2e socket lane — the fixture is load-bearing evidence for the `network-hls` row's e2e claim, not decorative.
## 2. What actually works (verified)
— `file` + `od` inspection (this audit); consumer call-sites verified by rg (hls_vod_e2e.rs:59-66, hls_attrs.rs:77, netfuzz_sweep.rs:127); the consuming e2e passes in the orchestrator baseline (4744 passed, AGENT-SPEC §13).
## 3-9. Findings / notes
None. Sizes consistent with the playlist (5 segments, 4136–8084 bytes); nothing suspicious (no embedded paths/secrets in the descriptor strings beyond "FFmpeg Service01").
