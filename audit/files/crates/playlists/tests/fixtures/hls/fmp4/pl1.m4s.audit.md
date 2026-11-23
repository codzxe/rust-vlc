# AUDIT: crates/playlists/tests/fixtures/hls/fmp4/pl1.m4s
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Type / size | `file` → "data" (generic); hex-inspected: genuine fragmented-MP4 media segment — `styp`(0x18) + `msdh`/`msix` brands + `sidx` + `moof`/`mfhd`/`trun` + `mdat`; 6 397 bytes |
| File verdict | REAL — genuine ffmpeg fMP4 segment bytes; its content is never asserted by any test (only referenced by name) |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file`; `od -A d -t x1` (box walk: offsets 0 `styp`, 80 `moof`, 160 `trun`, 352 `mdat`); `stat` → 6397; rg for `m4s` across crates/playlists/tests + crates/streaming/tests; ran `./target/debug/deps/hls_attrs-fbf9ea71a381ae2f fmp4` → `ffmpeg_hls_fmp4_map_matches_ffprobe_truth ... ok` (1 passed) |

## 1. Blunt verdict
Real fMP4 media segment (verified box structure by hex dump: styp/sidx/moof/mfhd/trun/mdat — the classic `-hls_segment_type fmp4` output), committed alongside the VERSION-7 playlist that names it. The one blemish: no test anywhere reads this file's bytes — the fmp4 suite asserts the playlist's EXTINF durations, the MAP stamping, and `init.mp4`'s length (764), but pl0–pl4.m4s are never size- or content-checked.

## 2. Evidence
- Hex: `00 00 00 18 73 74 79 70 6d 73 64 68 …` (styp, size 0x18, brand msdh), `73 69 64 78` at 24, `6d 6f 6f 66 00 00 00 10 6d 66 68 64` at 80 (moof+mfhd), `74 72 75 6e` at 160, `6d 64 61 74` at 352 — a structurally valid segment.
- Sibling playlist (pl.m3u8, VERSION 7, `#EXT-X-MAP:URI="init.mp4"`) lists `pl1.m4s` with `#EXTINF:1.996916`.
- The consuming test (`ffmpeg_hls_fmp4_map_matches_ffprobe_truth`, hls_attrs.rs:183) passes (I ran it) and cross-checks the family against ffprobe duration 9.046439.

## 3. Finding
F-001 [S4]
Where: crates/playlists/tests/fixtures/hls/fmp4/pl1.m4s (whole file) vs crates/playlists/tests/hls_attrs.rs:183-215
Code: (hls_attrs.rs:192 — the only fixture read in the fmp4 test)
```
    let init_len = fixture_len("fmp4", "init.mp4");
    assert_eq!(init_len, 764, "fixture must be the measured one");
```
What is wrong: only `init.mp4` is length-pinned; pl0–pl4.m4s (including this file) are never opened by any test — a truncated or swapped .m4s would change nothing in the suite. Harmless (the playlist parser only consumes the names), but the fixture set carries 25 KB of unverified binary ballast whose only role is realism.
Confidence: HIGH — Verification: read + rg + ran
