# AUDIT: crates/codecs/tests/fixtures/h264/F_still_1f_320x240.h264 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A05 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Type / size / sha256 | Raw H.264 Annex-B elementary stream ("data" — no container) / 4,430 bytes / 83472fc753768951ca6d45ce84af35ce2450c77217fc70c38c008bd2c855706c (md5 7542381db4fa8daeb324652d3ef2e11f) |
| File verdict | REAL (genuine single-IDR still, 320×240, Constrained Baseline — every filename claim verified) |
| Reachable from shipped entry point? | N/A (test fixture; the raw-ES h264 lane is the shipped CLI's open path for .h264 files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `stat`; `sha256sum`/`md5sum`; `ffprobe` (h264, Constrained Baseline, 320x240, yuv420p); python3 Annex-B NAL walk -> 4 NALs (1 SPS, 1 PPS, 1 SEI, 1 IDR slice); `./target/debug/deps/h264_sw_differential-9b87180403a77d06 lane_f_still` -> 2 passed / 0 failed (exit 0, 4.35s) |

## 1. One-paragraph verdict
A real, truthfully-named edge-case fixture, the ES twin of the already-audited
F_still_1f_320x240.mp4 (W06-A09): `file` reports headerless data (correct for
Annex-B); live ffprobe gives codec h264, profile Constrained Baseline,
320×240, yuv420p — matching the name; and my own NAL walk proves the "1f
still" claim structurally: exactly 4 NAL units = 1 SPS + 1 PPS + 1 SEI + 1
IDR slice (type 5) — a single-picture stream. Its documented purpose is the
flush-drain boundary (h264_sw_differential.rs:97-100 lane row `name:
"F_still_1f_320x240", frames: 1, size: (320, 240), yuv_md5:
"810d6f44c4f4e3b9eae14fd9aece2a76"`), and I executed both consumer lanes
live: `lane_f_still_annexb` (this file) and `lane_f_still_mp4` (the twin)
passed in 4.35s against the ffmpeg oracle — the ES lane asserts the
per-frame decode MD5 vs the pinned yuv_md5 while the stream finalizes at
flush (the lane's own comment: "The single-frame boundary: the picture
finalizes at flush", :440).

## 2. What actually works (verified)
- ffprobe facts above (100% filename agreement) + NAL census (1 slice).
- `./target/debug/deps/h264_sw_differential-9b87180403a77d06 lane_f_still`
  -> `lane_f_still_annexb ... ok`, `lane_f_still_mp4 ... ok` (2 passed,
  exit 0) — the fixture actively exercised, no skip.

## 3. Findings
None. Name, profile, dimensions, pixel format, and frame count all match
the probe and the NAL walk; the lane pins the flush/EOS boundary with a
pinned MD5 vs the live ffmpeg oracle.

## 4-9. (binary asset — short-form)
- Cited as evidence by: h264_sw_differential.rs lanes `lane_f_still_annexb`
  (:441) / `lane_f_still_mp4` (:446) — honest use for a genuine boundary
  case; no direct feature-matrix row citation found (the codec-video-h264
  evidence chain cites the differential suite as a whole).
- No in-tree digest of the container bytes (same posture as the other ES
  fixtures — the pinned yuv_md5 carries the regression value).
- Untouched on disk.
