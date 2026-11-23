# AUDIT: crates/codecs/tests/fixtures/h264/B_main_cabac_ip_320x240_45f.mp4 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Size / type | 58,689 bytes — `ISO Media, MP4 Base Media v1 [ISO 14496-12:2003]` |
| ffprobe facts | h264, profile Main, level 13, 320x240, yuv420p, nb_frames 45 (matches the lane table in h264_sw_differential.rs header: "B \| Main (CABAC, I/P) \| 45 \| 320x240") |
| md5 | 3a110ec34398e21525dd72f84212064b |
| Used by | crates/codecs/tests/h264_sw_differential.rs:73 (lane table), asserted at :370 (Annex-B twin) and :375 (this MP4 form) |
| Verdict | REAL — a genuine ffmpeg/libx264-authored fixture carrying a real Main/CABAC stream |

## Evidence
- `./target/debug/deps/h264_sw_differential-9b87180403a77d06 --nocapture` → **17 passed / 0 failed** (29.62s), including `lane_b_main_cabac_mp4 ... ok` and `lane_b_main_cabac_annexb ... ok` — the fixture decodes through the wired H264SwDecoder on BOTH entry forms and matches the ffmpeg oracle (per the suite's header: pinned MD5 re-verified live when ffmpeg is on PATH, which it was).
- The file is a well-formed MP4 (ffprobe parses it fully), dimensions/frames/profile exactly as the lane table documents.

## Notes
- The ES twin (`B_main_cabac_ip_320x240_45f.h264`, same dir) is the same bitstream re-framed (`-bsf:v h264_mp4toannexb` per the suite header).
- No doc/matrix row beyond h264_sw_differential.rs references this fixture by name (rg over crates/, docs/, tools/).
- No committed whole-file digest of the fixture itself found in-tree (the differential pins decoded MD5s, not the container bytes) — minor: a silently swapped fixture would still pass the live lane (same observation as the av1 audit's F-002).
