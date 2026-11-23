# AUDIT: crates/playlists/tests/fixtures/hls/fmp4/pl0.m4s
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Size / type | 6545 bytes / binary fMP4 media segment (ISOBMFF: styp msdh/msix + sidx v1 + moof) |
| File verdict | REAL fixture — bytes never read by any test |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file` (data); `od -A d -t x1z` (box structure); wc -c; rg (consumers) |

## 1. Blunt verdict
Genuine ffmpeg-generated fMP4 media segment, not a fabricated blob. Box structure
hexdump-verified: `styp` (brands msdh/msix), `sidx` v1, `moof`+`mdat`. Internally
consistent with the sibling playlist pl.m3u8: sidx timescale = 22050 (22.05 kHz AAC),
subsegment_duration = 0xB000 = 45056 ticks = 2.0434 s == EXTINF 2.043356 for pl0.
The catch: NO test reads these bytes — pl.m3u8 references it as a URI string
(hls_attrs.rs:776 builds "https://cdn.example/vod/fmp4/pl0.m4s" as an expected
resolution) and the netfuzz sweep reads only the .m3u8 text; only init.mp4 (764 bytes,
pinned at hls_attrs.rs:193) is length-checked. If any evidence row cites pl0.m4s as
proof of fMP4 segment HANDLING, that is inflation — the playlists crate parses text,
nothing parses this segment.

## 2. Evidence
— `od -A d -t x1z -v` first 80 bytes: `00 00 00 18 73 74 79 70 6d 73 64 68 ...`
(styp), `00 00 00 34 73 69 64 78 01 ...` (sidx v1, timescale 00 00 56 22 = 22050),
`6d 6f 6f 66` (moof) at offset 84.
— rg across crates/apps/tests: `pl0.m4s` appears only in pl.m3u8 and as a URL literal
in hls_attrs.rs:776. Zero byte-reads.
— Family: pl0-pl4.m4s (6545/6397/6335/6335/3481 B) + init.mp4 (764 B) + pl.m3u8; the
playlist's 5 EXTINFs sum to 9.04644 s, asserted vs ffprobe 9.046439 at 0.01 tolerance
(hls_attrs.rs:206-209, passing in baseline PACKAGE playlists).

## 3. Finding
F-001 [S4] Fixture bytes unconsumed by any test — inert realism. Confidence: HIGH —
Verification: read (rg exhaustive) + od.
