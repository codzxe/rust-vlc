# AUDIT: crates/codecs/tests/fixtures/h264/F_still_1f_320x240.mp4 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| File verdict | REAL (genuine single-frame H.264 MP4; every filename claim verified true) |
| Reachable from shipped entry point? | N/A (test fixture; mp4+h264 is the CLI's primary shipped lane) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `ffprobe`, rg + od citation checks; baseline cite |

## 1. One-paragraph verdict
A real, truthfully-named edge-case lane: `file` reports ISO Media MP4; live ffprobe gives `codec_name=h264, profile=Constrained Baseline, width=320, height=240, pix_fmt=yuv420p, nb_frames=1, duration=0.033333` — still (1 frame), 320×240, exactly as the filename says. Its documented purpose is the flush-drain boundary (h264_sw_differential.rs:19 — "single-IDR still | 1 | 320x240 | the flush-drain boundary"), and both entry forms are pinned: `lane_f_still_annexb` (:441) and `lane_f_still_mp4` (:446) passed in the baseline (:1177-1178, part of the 17/17 / 24.74s live-oracle run). The ES twin `F_still_1f_320x240.h264` is committed alongside.

## 2. What actually works (verified)
- `file` + `ffprobe` outputs above (pasted in run log) — 100% filename/name-field agreement.
- Citations (od-verified past display mangling): lane table :97 `name: "F_still_1f_320x240"`, call sites :441/:446.
- Baseline :1177-1178: `lane_f_still_annexb ... ok`, `lane_f_still_mp4 ... ok`.

## 3. Findings
None. Name, profile, dimensions, pixel format, and frame count all match the probe; the lane pins the flush/EOS boundary with a bit-exact MD5 vs ffmpeg's decode.

## 4-9. (binary asset — short-form)
- Cited as evidence by: h264_sw_differential.rs (lanes F annexb+mp4, bit-exact vs the ffmpeg oracle) — honest use for a genuine boundary case.
- No direct matrix-row citation; untouched on disk.
