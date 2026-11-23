# AUDIT: crates/codecs/tests/fixtures/h264/B_main_cabac_ip_320x240_45f.h264 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| File verdict | REAL |
| Type / size | Raw H.264 Annex-B elementary stream (starts `00 00 00 01` + SEI NAL type 6 with x264 encoder string "x264 - core 164 r3108"), 57693 bytes — `file` reports "data" (no magic for raw ES; verified by my byte walk) |
| Structure (verified by my python walk) | 48 NALs: types {1, 5, 6, 7, 8} — SEI/SPS/PPS + **exactly 45 slice NALs** (matches "45f"); SPS at byte 630: profile_idc 0x4d = **77 = Main** ✓ (matches "main"), constraints 0x40, level 0x0d = 1.3 |
| Cited as evidence by | tests/h264_sw_differential.rs:73,370,375 (ES + MP4 lanes); fixtures/h264 family used across parser-corpus suites |
| Commands run | `file`; python3 NAL walk (output above); `./target/debug/deps/h264_sw_differential-9b87180403a77d06` → 17 passed (exit 0, 40 s, includes lane_b_main_cabac_annexb + lane_b_main_cabac_mp4 full decode differentials); `./target/debug/deps/h264_parser_corpus-bf10919714fec14e` → 2 passed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Genuine x264-authored Annex-B stream whose name is accurate on every axis I could byte-verify: Main profile (SPS profile_idc 77), 45 slice NALs, SEI/SPS/PPS present, CABAC exercised by the passing differential lanes. Both consuming suites (17-test sw differential vs ffmpeg-built reference, 2-test parser corpus vs ffprobe traces) pass under my execution. No findings.
