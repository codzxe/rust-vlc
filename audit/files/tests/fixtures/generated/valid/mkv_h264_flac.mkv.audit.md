# AUDIT: tests/fixtures/generated/valid/mkv_h264_flac.mkv
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 33443 B / c2f798f0b40d800d231ebfc01d3fb1c03365c80294287cdfeefe7373bb866dd3 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/mkv_fixtures.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (data audit)
Genuine Matroska: EBML magic `1a45dfa3` verified; it is the muxed h264-video + flac-audio lane named in the suite header (`mkv_h264_aac.mkv`, `mkv_h264_flac.mkv` — muxed A/V lanes, mkv_fixtures.rs:11) and a fixture row at mkv_fixtures.rs:126 (`("mkv_h264_flac.mkv", 2)` — 2 tracks). The consuming suite is unusually strong: `valid_fixtures_match_ffprobe_packet_counts` and `audio_payload_is_bit_exact_vs_ffmpeg` — the ffprobe/ffmpeg oracle is inside the test itself, so this fixture IS externally-oracle-judged even though it sits outside the .oracle.sha256 sidecar perimeter.

## 2. What was checked and how
- batch.py: 33443 B, sha256 c2f798f0…, EBML magic confirmed (`magic_is_ebml: True`).
- Consumers: crates/demux/tests/mkv_fixtures.rs:11, :126, :155.
- Execution: audit/runs/baseline/cargo-test-workspace.txt:2353-2367 — mkv_fixtures 9/9 ok, including the two ffprobe/ffmpeg-oracle tests.
- Generated-vs-handmade: xtask-generated family; NOT in MANIFEST.toml (python cross-check).

## 3. Findings
F-001 [S3] Not covered by the MANIFEST.toml drift gate (28/117 pinned files; mkv family absent). Verification: ran (python manifest cross-check).
