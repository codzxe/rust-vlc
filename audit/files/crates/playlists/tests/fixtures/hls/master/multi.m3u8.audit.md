# AUDIT: crates/playlists/tests/fixtures/hls/master/multi.m3u8
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Type / size | `file` → "M3U playlist, ASCII text"; 572 bytes, 9 lines (LF) |
| File verdict | REAL — hand-shaped multi-variant HLS master; every attribute line is asserted field-by-field by a passing test, and the file doubles as a netfuzz seed |
| Reachable from shipped entry point? | N/A (test fixture); the master parser it exercises is library code (card 6.B2a selection policy lives in streaming, also unwired — see media.rs audit F-002) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `cat -A` (9 LF lines); `stat` → 572; `./target/debug/deps/playlists-c262b72361161a1c master` → 6 passed incl. `master_types_variants_and_renditions` (exit 0); `./target/debug/deps/netfuzz_sweep-179086f8a3bb1747` → 2 passed / 2 ignored-60s (exit 0) |

## 1. Blunt verdict
Exactly what it claims: a multi-variant master (2 `#EXT-X-MEDIA` renditions — audio "English"/en + subtitles "Svenska"/sv — and 3 `#EXT-X-STREAM-INF` rungs 2 000 000/800 000/200 000 with AVERAGE-BANDWIDTH, PROGRAM-ID, CODECS, RESOLUTION; the lowest rung deliberately omits the optional attributes). The consuming test (hls.rs:866 `master_types_variants_and_renditions`) asserts every field of variant 0, the absent resolution of variant 2, and both renditions' kinds/flags/languages/resolved URIs — and passes (I ran it). The file is also one of the six real HLS seeds in the netfuzz sweep (netfuzz_sweep.rs:124), which I ran (survival tests pass).

## 2. Evidence
- Fixture↔test correspondence: `BANDWIDTH=2000000,AVERAGE-BANDWIDTH=1950000,PROGRAM-ID=1,CODECS="avc1.640028,mp4a.40.2",RESOLUTION=1280x720` ↔ `bandwidth == 2_000_000`, `average_bandwidth == Some(1_950_000)`, `program_id == Some(1)`, codecs/resolution equality (hls.rs:873-886).
- Rendition flags: `DEFAULT=YES,AUTOSELECT=YES` on audio, neither on subtitles ↔ `is_default && autoselect` / `!is_default` (hls.rs:889-891).
- Execution: `playlists … master` → 6 passed; `netfuzz_sweep` → `netfuzz_sweep_hls_playlist_survives … ok` (this fixture among the seeds).
- Cited as a fuzz corpus seed per netfuzz_sweep.rs module docs ("the same files are the committed fuzz/corpus/ seeds") — consistent with fuzz/corpus/fuzz_hls_playlist/ contents.

## 3. Notes
No defects. The variant-2 attribute omissions (no AVERAGE-BANDWIDTH/PROGRAM-ID/RESOLUTION) are deliberate negative coverage for Option handling — verified by the `master.variants[2].resolution == None` assertion.
