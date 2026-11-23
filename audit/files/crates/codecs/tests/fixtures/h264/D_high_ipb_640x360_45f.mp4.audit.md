# AUDIT: crates/codecs/tests/fixtures/h264/D_high_ipb_640x360_45f.mp4 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| File verdict | REAL (valid fixture, actively used as differential evidence) |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> ISO Media, MP4 Base Media v1; `ls -la` -> 147583 bytes; rg citations |

## 1. One-paragraph verdict
Real ffmpeg-authored MP4 fixture (High-profile I/P/B, 640x360, 45 frames; sibling .h264 Annex-B form alongside). Used as one of six bit-exact differential lanes by crates/codecs/tests/h264_sw_differential.rs (`lane_d_high_ipb_mp4`, L425) with a pinned `yuv_md5` (412bd350f062af2e35cffb45cfec7f9b) and pinned frame count/size; both D-lane tests pass in the baseline (cargo-test-workspace.txt:1174-1175). No defects.

## 2. What actually works (verified)
— verified by `file crates/codecs/tests/fixtures/h264/D_high_ipb_640x360_45f.mp4` -> "ISO Media, MP4 Base Media v1 [ISO 14496-12:2003]"; size 147583 bytes.
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:1174-1175 -> lane_d_high_ipb_annexb ok, lane_d_high_ipb_mp4 ok.

## 3. Findings
None. Checked: file type sane for claimed content; cited as evidence by a real test that asserts pinned digests (a tampered fixture would fail the md5 pin); no doc/matrix row mis-cites it.

## 4. Per-function table
N/A (binary asset).

## 5-9. (n/a for binary assets)
Cited by: crates/codecs/tests/h264_sw_differential.rs:85 (lane def), :420, :425 (tests). Provenance claim (ffmpeg/libx264-authored, six lanes) lives in that test's docs — UNVERIFIED here (regenerating requires ffmpeg; the pinned md5s are the tamper guard).
