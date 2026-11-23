# AUDIT: crates/playlists/tests/fixtures/hls/fmp4/pl2.m4s (binary fixture)
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Size / type | 6335 bytes; `file` → "data"; `od` shows a well-formed fMP4 media segment: `styp` box (brands msdh/msix), `sidx` (segment index, earliest_presentation 0x5622, first_offset 0x15c), then `moof` — FFmpeg `-hls_segment_type fmp4` output |
| File verdict | REAL (genuine fMP4 segment) — but playlist-level evidence only: no test in the tree parses its bytes |
| Reachable from shipped entry point? | N/A (test asset). Byte-level consumers: NONE — `rg "fmp4"` in crates/streaming/tests/{hls_vod_e2e,hls_conformance_e2e}.rs → 0 hits; hls_attrs.rs uses only its existence (read_dir count, :77) and sibling lengths (init.mp4 = 764 B, :192); netfuzz_sweep.rs:128 parses the fmp4 PLAYLIST text, not the segment. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. Verdict
A real FFmpeg-fMP4 segment backing the fmp4 playlist (`#EXT-X-VERSION:7` + `#EXT-X-MAP:URI="init.mp4"`, EXTINF 1.996916 for pl2), which in turn backs the hls_attrs ffprobe truth table (duration 9.046439) and the matrix's M3U/M3U8 evidence (formats row, docs/feature-matrix.toml:1168 "EXTINF sums vs ffprobe 9.046543/9.046539"). The bytes themselves are never demuxed anywhere: the in-tree MP4 demuxer is never fed this segment, and the HLS session lane REFUSES the whole fmp4 playlist typed (`streaming.hls.init-segment`, live.rs/media.rs refusal tables) — consistent with the matrix note "fMP4 lanes refuse typed (streaming.hls.*)" (docs/feature-matrix.toml:1442). So the fixture proves the PARSER's truth table, nothing beyond it.
## 2. What actually works (verified)
— `file`/`od` box-level inspection (styp/sidx/moof present and coherent); consumers verified by rg (existence/count/playlist-text only); consuming tests pass in the baseline.
## 3-9. Findings / notes
F-001 [S4]: the segment bytes are dead weight as bytes — an fMP4-decode test (mp4 demuxer over init.mp4 + pl2.m4s) is the obvious missing evidence the network-hls "fMP4 refuse" posture makes impossible today. No misrepresentation found: no test or doc claims these bytes are parsed.
