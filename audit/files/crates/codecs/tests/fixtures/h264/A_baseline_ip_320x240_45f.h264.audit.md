# AUDIT: crates/codecs/tests/fixtures/h264/A_baseline_ip_320x240_45f.h264
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 59,909 bytes (binary) / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test fixture; lane A of the SW differential that validates the SHIPPED rust_h264 decoder — the strongest h264 evidence chain) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> data (raw Annex-B ES); python3 Annex-B NAL scan + full SPS bit-decode (below); `rg` consumer map (5 files); baseline citation audit/runs/baseline/cargo-test-workspace.txt:1163-1182 |

## 1. One-paragraph verdict
Genuine raw Annex-B H.264 elementary stream, and every element of its filename is provable from the bytes: 48 NALs = 1 SEI + 1 SPS + 1 PPS + 1 IDR slice (type 5) + 44 non-IDR P slices (type 1) = 45 frames "ip" (one I, 44 P, zero B); the SPS decodes (my independent bit-walk) to profile 66 with constraint flags 0xC0 (constraint_set0+set1 = **Constrained Baseline**), level_idc 13 (1.3), sps_id 0, poc_type 2, 20×15 macroblocks = **320×240** coded, no cropping, VUI present with aspect_ratio_idc 1 (1:1). It is lane A of `h264_sw_differential.rs`, which pins the whole-stream yuv420p MD5 (`a0a9144b154d0403aa38429a85adce56`, ffmpeg 7.1.5) on BOTH entry forms (Annex-B ES and MP4-wrapped) — and that suite passed 17/17 in the baseline in 24.74s (a real decode run, not a skip).

## 2. What actually works (verified)
— python3 NAL scan: `NAL count 48 types {6: 1, 7: 1, 8: 1, 5: 1, 1: 44}` — exactly one I + 44 P.
— python3 SPS decode: `profile 66 cons 0xc0 level 13 sps_id 0 ... width_mbs-1 19 height_map-1 14 frame_mbs_only 1` → coded 320x240; `cropping flag 0`; `vui 1`, `aspect_ratio_idc 1` (after fixing my own first-pass decode which had omitted direct_8x8_inference_flag — noted so nobody re-derives the wrong offsets).
— Baseline: `lane_a_baseline_annexb ... ok` / `lane_a_baseline_mp4 ... ok` (audit/runs/baseline/cargo-test-workspace.txt:1168-1169) inside the 17/17 suite run (finished in 24.74s).

## 3. Findings
None. Checked: NAL type distribution vs the name, SPS identity vs the name, frame count vs the name, start-code framing integrity (48 well-formed NALs, last NAL runs to EOF), emulation-prevention bytes survive round-trip in my unescaper.

## 4. Per-function table
N/A (binary asset).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| h264_sw_differential lane_a_baseline_annexb / _mp4 | YES | pinned ffmpeg-decode MD5 of the full 45-frame yuv420p output, both container forms; any decode deviation changes the digest; ran in baseline |
| hostile lanes reusing this fixture's packets (garbage-then-real, truncation sweep) | YES | recovery/no-panic properties on this file's bytes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bit-exact vs ffmpeg 7.1.5 on six lanes, both entry forms" (h264 SW admission) | crates/codecs/src/video/h264/mod.rs:20-22 | REAL | lane tests with pinned MD5s passed in baseline (17/17, 24.74s); ffmpeg itself not re-run here (UNVERIFIED directly) |

## 7. Dead / duplicated / unreachable code
N/A. Referenced by 5 test files (codecs differential + apps CLI tests).

## 8. Dependencies & flags
Data-only asset (59,909 bytes matches manifest row).

## 9. Open questions for the next agent
None.
