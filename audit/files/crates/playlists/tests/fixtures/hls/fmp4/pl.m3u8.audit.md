# AUDIT: crates/playlists/tests/fixtures/hls/fmp4/pl.m3u8
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Type / size | `file`: M3U playlist, ASCII text — 269 bytes, 17 lines |
| Verdict | REAL fixture (actively consumed; ffprobe-truth cross-check pinned) |
| Cited as evidence? | YES — `hls_attrs.rs:17` ("fMP4 + MAP … duration 9.046439, `init.mp4` = 764 bytes"); also a netfuzz sweep seed (netfuzz_sweep.rs:128). Not directly named in feature-matrix.toml (the hls-playback row's evidence cites the suites that consume it). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

Content verified: `#EXTM3U`, `#EXT-X-VERSION:7`, `#EXT-X-TARGETDURATION:2`, `#EXT-X-MEDIA-SEQUENCE:0`, `#EXT-X-PLAYLIST-TYPE:VOD`, `#EXT-X-MAP:URI="init.mp4"`, five `#EXTINF` + segment pairs (pl0..pl4.m4s; durations 2.043356 / 1.996916 ×3 / 1.012336), `#EXT-X-ENDLIST`. EXTINF sum = 9.046440 — matches the ffprobe-measured 9.046439 the test cross-checks (hls_attrs.rs:183-215, tolerance 0.01, executed green in the suite baseline).
Consumers: `hls_attrs.rs:30` (`include_str!` FMP4 const — `ffmpeg_hls_fmp4_map_matches_ffprobe_truth` pins 5 entries, MAP stamped on every segment, exact per-entry durations); `netfuzz_sweep.rs:128` (real-fixture fuzz seed — executed by me, 3000 iterations). The companion binaries (init.mp4 = 764 B verified; pl0-pl4.m4s) are structurally consistent with the ffmpeg fMP4 lane but only init.mp4's length is asserted (see pl3.m4s audit).
No fabrication indicators: durations are non-round, ffmpeg-flavored (flush-tail 1.012336 on the last segment), and pinned against an external tool's measurement.
Commands run: `file` (M3U), `stat` (269 B), full read, grep consumers, EXTINF sum re-computed.
