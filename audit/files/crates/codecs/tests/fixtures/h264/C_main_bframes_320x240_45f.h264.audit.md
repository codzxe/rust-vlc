# AUDIT: crates/codecs/tests/fixtures/h264/C_main_bframes_320x240_45f.h264 (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Size / type | raw Annex-B H.264 elementary stream (`file`: data; ffprobe parses it fully) |
| ffprobe facts | h264, profile Main, level 13, 320x240, yuv420p, has_b_frames 2 (matches lane table "C \| Main + B-frames (bf=2) \| 45 \| 320x240") |
| md5 | e268f8b1b63954465e5452f1e1b7e18c |
| Used by | crates/codecs/tests/h264_sw_differential.rs:79 (lane table), asserted at :384 (this ES form), :397 (MP4 twin) |
| Verdict | REAL — a genuine B-frame-carrying libx264 stream for the reorder lane |

## Evidence
- `./target/debug/deps/h264_sw_differential-9b87180403a77d06` → 17/17 pass including `lane_c_bframes_annexb ... ok` and `lane_c_bframes_mp4 ... ok` (the ffmpeg-oracle MD5 comparison, live).
- The lane's extra assertions executed: ES frames echo `Stamp::Missing` (documented raw-ES posture, asserted per-frame at :391); the MP4 twin's pts are asserted strictly increasing (display order after reorder).
- has_b_frames=2 confirms the fixture actually exercises reordering, not a re-encoded I/P stream mislabeled "bframes".

## Notes
- Same no-whole-file-digest observation as the other h264 fixtures (decoded MD5s pinned, container bytes not).
- No other doc/matrix references to this fixture by name.
