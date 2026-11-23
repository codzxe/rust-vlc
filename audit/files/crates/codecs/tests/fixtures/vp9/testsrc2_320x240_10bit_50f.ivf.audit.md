# AUDIT: crates/codecs/tests/fixtures/vp9/testsrc2_320x240_10bit_50f.ivf (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| File verdict | REAL (genuine 10-bit VP9 IVF fixture; consumed by a passing baseline differential) |
| Reachable from shipped entry point? | N/A (test fixture); the IVF container lane it exercises IS CLI-reachable (`containers::ivf`), but this exact file is only opened by the codecs test suite |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `ffprobe`, rg citing tests (exit 0); baseline cite |

## 1. One-paragraph verdict
A real, correctly-named binary asset. `file` reports "Duck IVF video file, codec VP90, 320x240, 50 frames"; live ffprobe confirms: `codec_name=vp9, profile=Profile 2, pix_fmt=yuv420p10le, 320x240, r_frame_rate=25/1, nb_frames=50, duration=2.0` — the filename's claims (testsrc2, 320x240, 10bit, 50f) are all true (Profile 2 is the 10-bit VP9 profile; yuv420p10le is the 10-bit format). 83,565 bytes. It is consumed by `crates/codecs/tests/vp9_differential.rs` as `TENBIT_LANE` (:102), where `tenbit_lane_is_a_typed_refusal_not_a_misdecode` (:333-352) asserts all 50 packets refuse with the exact typed code `vp9.bitdepth` and zero fabricated frames — that test passed in the baseline (:1390 area, 4/4 ok in 21.12s, which includes live ffmpeg-oracle lanes). The test's eprintln honestly defers engine-level 10-bit bit-exactness to the admission record (tools/vta/vp9, MD5 2391b424…).

## 2. What actually works (verified)
- `file` → "Duck IVF video file, codec VP90@\001\360, 320x240, 50 frames".
- `ffprobe` → vp9 / Profile 2 / yuv420p10le / 320x240 / 25 fps / 50 frames / 2.0 s (output pasted in run log).
- Baseline cargo-test-workspace.txt:1384-1394: vp9_differential 4/4 ok incl. `tenbit_lane_is_a_typed_refusal_not_a_misdecode`.

## 3. Findings
None. Fixture dimensions, frame count, bit depth, and codec all match the filename and the consuming test's assertions (50 packets asserted).

## 4-9. (binary asset — per spec §12, short-form audit)
- Cited as evidence by: vp9_differential.rs TENBIT_LANE (typed-refusal pin) — the citation is honest (the fixture proves the refusal boundary, not decode quality).
- Doc/matrix rows touching it: none found outside the test and the vp9 admission-record reference.
- No modification, no signature verification beyond `file`/ffprobe consistency.
