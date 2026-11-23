# AUDIT: crates/playlists/tests/fixtures/hls/single_file/pl.m3u8 (fixture)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Type / size | M3U playlist, ASCII text, 368 bytes (sibling `pl.ts` = 35 344 bytes, verified on disk) |
| File verdict | REAL (real ffmpeg `-hls_flags single_file` output, verified against disk truth by its consumer test) |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `wc -c` (pl.m3u8 + pl.ts), full read, consumer scan (hls_attrs.rs, netfuzz_sweep.rs) |

## 1. Verdict
A 5-segment single-file BYTERANGE media playlist (EXT-X-VERSION:4, TARGETDURATION:2, PLAYLIST-TYPE:VOD, ENDLIST) whose ranges tile the sibling `pl.ts` exactly: offsets 0/8084/15792/23500/31208 with lengths 8084/7708/7708/7708/4136 → 31 208 + 4 136 = 35 344 = the on-disk file length (verified by `wc -c`). The consumer test (`hls_attrs.rs:126` `ffmpeg_hls_single_file_byteranges_match_disk_truth`) re-derives contiguity and total coverage from disk live rather than trusting constants, and asserts the EXTINF sum 9.046544 against the recorded ffprobe duration 9.046543 within 0.01 s. Also consumed by `netfuzz_sweep.rs:129` (cross-crate fuzz corpus).

## 2. What actually works (verified)
— Arithmetic by hand (offsets contiguous, exact file tiling); `wc -c pl.ts` -> 35344; consumer test exists and is exact (would fail if any range value or the file changed).

## 3. Findings
None. The fixture is internally consistent and its test checks it against disk truth, not against itself.

## 4-9. —
Per-function table N/A. Test honesty: consumer asserts disk-derived facts (strong). Doc claims: hls_attrs.rs header table row "single-file byteranges … duration 9.046543, pl.ts = 35344 bytes" — matches disk. Dead code: none. Dependencies: sibling pl.ts. Open questions: none.
