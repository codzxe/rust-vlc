# AUDIT: crates/playlists/tests/fixtures/hls/single_file/pl.ts
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Type / size | MPEG transport stream data (`file`), 35 344 bytes |
| File verdict | REAL — genuine ffmpeg-generated TS payload, consumed as disk-truth by a passing test |
| Reachable from shipped entry point? | N/A (test fixture); the parser it feeds is library-code (playlists crate), not wired to any shipped host |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → MPEG transport stream data; `od -A d -t x1` (starts `47 40 11 10 …` — TS sync 0x47 + PID 0x4011, real PAT/PMT head); `stat` → 35344; `./target/debug/deps/hls_attrs-fbf9ea71a381ae2f single_file` → `ffmpeg_hls_single_file_byteranges_match_disk_truth ... ok` (1 passed, exit 0) |

## 1. Blunt verdict
Real binary fixture, not a placeholder: a 35 344-byte MPEG-TS whose five `#EXT-X-BYTERANGE` entries in the sibling `pl.m3u8` (8084+7708+7708+7708+4136) tile it exactly to the last byte. The consuming test derives the tiling live from disk (`fixture_len`) rather than trusting a constant, so a truncated or regenerated file fails loudly. Verified by execution.

## 2. Evidence
- `od` head: `0000000 47 40 11 10 00 42 f0 25 …` — TS sync byte 0x47 at offset 0, PAT/PMT structure; `file` says MPEG transport stream data.
- Sum check: 8084+7708+7708+7708+4136 = 35344 == `stat` size. The playlist's last byterange offset 31208 + 4136 = 35344.
- Test run: `hls_attrs … single_file` → 1 passed, 0 failed.
- Cited as evidence by feature-matrix.toml:1167 (network-hls row: "EXTINF sums vs ffprobe 9.046543/9.046539") — the duration sum 9.046543 is asserted live in the test against this fixture family.

## 3. Notes
No defects. The fixture is committed (mtime matches the fixture batch); no doc/matrix row cites pl.ts by name beyond the hls_attrs header table (hls_attrs.rs:16), which matches reality.
