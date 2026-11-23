# AUDIT: crates/codecs/tests/fixtures/h264/D_high_ipb_640x360_45f.h264 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | raw H.264 Annex-B ES (`file`: "data"; ffprobe: h264 High 640x360 yuv420p, 45 frames) / 146319 bytes / 2bf20d62120faccf258bf3ff35d9a8f40f0e53bf5c2390ea7f75c786a370e258 |
| File verdict | REAL (differential decode evidence, executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `sha256sum`; `ffprobe -count_frames` -> h264/High/640x360/yuv420p/45 frames; `./target/debug/deps/h264_sw_differential-9b87180403a77d06 lane_d_high_ipb --nocapture` -> 2 passed (annexb + mp4 forms), 10.20s |

## 1. Verdict
Real High-profile IPB fixture, one of six lanes in the `rust_h264` software-decoder admission suite (`crates/codecs/tests/h264_sw_differential.rs`). Not a decoration: both entry-form tests for this lane decode the full 45 frames through the shipped `H264SwDecoder` path and verify a bit-exact YUV MD5 against a committed ffmpeg 7.1.5 golden (`412bd350f062af2e35cffb45cfec7f9b`, h264_sw_differential.rs:88), plus a live ffmpeg re-check when ffmpeg is on PATH (it is here).

## 2. Test usage (verified by execution)
`lane_d_high_ipb_annexb` + `lane_d_high_ipb_mp4` (h264_sw_differential.rs:418-426) both **passed** in 10.20s — real decode work (frame count, per-frame size/format, concatenated YUV MD5 vs golden, live ffmpeg oracle re-check). This is the strongest form of fixture evidence in the repo: independent-oracle bit-exactness, not self-generated goldens.

## 3. Matrix evidence
The fixture backs the sw.rs admission record ("six ffmpeg-authored lanes … bit-exact against ffmpeg 7.1.5", sw.rs:14-19) and is cited by name in docs/dependencies/records/rust_h264-0.6.1.md per that doc comment — consistent with the executed result. No inflation detected for this asset.
