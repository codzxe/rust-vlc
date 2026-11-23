# AUDIT: crates/playlists/tests/fixtures/hls/ts_vod/pl.m3u8
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 16 / fixture (media playlist) + 5 sibling .ts segments |
| File verdict | REAL (ffmpeg-generated HLS fixture with on-disk segment parity and an ffprobe truth anchor) |
| Reachable from shipped entry point? | N/A — test fixture; consumers: crates/playlists/tests/hls_attrs.rs:28 (`include_str!`), crates/streaming/tests/netfuzz_sweep.rs:127 (cross-crate fuzz seed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file` → "M3U playlist, ASCII text"; `file pl0.ts` → "MPEG transport stream data"; python3 sync-byte check (all 5 segments start 0x47); EXTINF sum = 9.046544; `cargo test -p playlists --test hls_attrs` → 14 passed; `cargo test -p streaming --test netfuzz_sweep` → 2 passed/2 ignored |

## 1. One-paragraph verdict
A genuine ffmpeg-produced media playlist: `#EXT-X-VERSION:3`, `TARGETDURATION:2`, `MEDIA-SEQUENCE:0`, `PLAYLIST-TYPE:VOD`, five `#EXTINF` entries (2.043356, 1.996922, 1.996911, 1.996922, 1.012433) pointing at `pl0.ts`…`pl4.ts`, closed by `#EXT-X-ENDLIST`. All five sibling .ts files exist in the same directory, are real MPEG transport streams (sync byte 0x47, sizes 8084/7708/7708/7708/4136 — durations visibly proportional to the EXTINF values). The consuming test (`ffmpeg_hls_ts_vod_matches_ffprobe_truth`, hls_attrs.rs:71-121) does honest work: it derives the expected entry count from the directory listing rather than a constant, asserts each EXTINF duration verbatim, checks the sum against the recorded ffprobe duration 9.046543 within 0.01 s, asserts URI order, header-tag preservation, ENDLIST/VOD semantics, and the roundtrip law. Both suites consuming it ran green. TARGETDURATION:2 is legal (2.043356 rounds to 2).

## 2. What actually works (verified)
— `cargo test -p playlists --test hls_attrs` → `14 passed; 0 failed`; `cargo test -p streaming --test netfuzz_sweep` → `2 passed; 2 ignored` (fixture is a fuzz seed there). — Segment/playlist parity: 5 EXTINF entries ↔ 5 .ts files on disk (test asserts this dynamically). — EXTINF sum 9.046544 vs ffprobe 9.046543 (python3): inside the test's 0.01 s tolerance, and the test's comment correctly attributes the 1 µs to ffmpeg's own EXTINF rounding.

## 3. Findings
F-001 [S4]
Where: crates/playlists/tests/fixtures/hls/ts_vod/pl.m3u8 vs crates/streaming/tests/hls_vod_e2e.rs:225-233
Code:
```
#EXTINF:2.043356,
pl0.ts
```
while the streaming e2e's wire-shape assertion expects:
```
        "/hls/ts_vod/seg000.ts",
        "/hls/ts_vod/seg001.ts",
```
What is wrong: nothing in the fixture — but there are TWO unrelated `ts_vod` fixture lanes in the tree (this one, `pl0..4.ts`, under crates/playlists/tests/fixtures/hls/ts_vod/; and a separate one, `seg000..003.ts`, under the workspace root tests/fixtures/hls/ts_vod/ resolved via `testkit::fixture_bytes`). Same directory name, same `pl.m3u8` filename, different segment names and counts (5 vs 4). A maintainer editing "the ts_vod fixture" can trivially edit the wrong one; the streaming e2e does not consume this file. Why it matters: fixture-name collision invites cross-edit mistakes; the netfuzz_sweep comment ("read cross-crate read-only") is the only thing signaling which is which. What correct looks like: distinct lane names (e.g. `ts_vod_pl` vs `ts_vod_seg`) or a README in each fixture dir naming its consumers. Confidence: HIGH — Verification: read both directories + both consuming tests.
(The fixture itself is otherwise consistent and fully asserted; no fixture defect.)

## 4-9. (fixture — per-function table N/A; no tests inside; the hls_attrs test IS the doc-claim check: "TS VOD | fixtures/hls/ts_vod/pl.m3u8 | ffprobe duration 9.046543, 5 segments on disk" — verified true; no dead content; no dependencies; open question covered by F-001)
