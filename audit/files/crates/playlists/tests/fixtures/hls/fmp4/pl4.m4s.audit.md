# AUDIT: crates/playlists/tests/fixtures/hls/fmp4/pl4.m4s
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Size / type | 3481 bytes / binary fMP4 media segment (ISOBMFF: styp msdh/msix + sidx v1 + moof) |
| File verdict | REAL fixture — bytes never read by any test |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file` (data); `od -A d -t x1z` (box structure, head+tail); wc -c; rg (consumers) |

## 1. Blunt verdict
Genuine ffmpeg-generated fMP4 media segment, final (short) segment of the family, and
numerically consistent with the playlist: sidx v1 earliest_presentation_time =
0x0002B400 = 177152 ticks @ 22050 Hz = 8.0340 s == sum of pl0-pl3 EXTINFs
(2.043356 + 3×1.996916 = 8.034104 s); subsegment_duration = 0x5732 = 22322 ticks =
1.0123 s == EXTINF 1.012336. Tail bytes are payload (no padding artifact). Same
defect as pl0: no test reads these bytes — URI-string references only; nothing in the
tree demuxes this segment, so as "evidence" it is inert.

## 2. Evidence
— `od` head: identical styp/sidx-v1 header shape to pl0; ept field at bytes 44-51 =
00 00 00 00 00 02 b4 00; duration field = 00 00 57 32.
— `od` tail (offset 3472-3480): `19 56 e4 30 65 5b 8a 82 6e` — payload to EOF.
— rg: `pl4.m4s` appears only inside pl.m3u8. Zero byte-reads anywhere.
— Playlist arithmetic cross-checked by hand (values above); family total 9.04644 s
asserted vs ffprobe 9.046439 in hls_attrs.rs (passing in baseline).

## 3. Finding
F-001 [S4] Fixture bytes unconsumed by any test — inert realism; only its playlist
metadata (EXTINF) is ever asserted. Confidence: HIGH — Verification: read (rg) + od.
