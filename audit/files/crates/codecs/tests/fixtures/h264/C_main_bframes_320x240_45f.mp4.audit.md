# AUDIT: crates/codecs/tests/fixtures/h264/C_main_bframes_320x240_45f.mp4
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 55,978 bytes (binary) / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test fixture; lane C (MP4 form) of the h264 SW differential validating the shipped rust_h264 decoder + the MP4/avcC framing path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> ISO Media, MP4 Base Media v1; python3 box walk + avcC/SPS/stts/stss parse + mdat slice-type scan (below); baseline citation audit/runs/baseline/cargo-test-workspace.txt:1163-1182 |

## 1. One-paragraph verdict
Genuine MP4-wrapped H.264 stream, and the filename is provable from the bytes end-to-end: box tree ftyp(MP4 v1)/free/mdat 54,682 B/moov(mvhd,trak(tkhd,edts/elst,mdia(...stbl(stsd(avc1))))) is well-formed; the `avc1` entry says 320×240 with an `avcC` record (version 1, profile 77 = **Main**, constraint byte 0x40, level 13, 4-byte NAL length size, 1 SPS + 1 PPS, plus pasp/btrt children); the SPS (my independent bit-walk) is Main@1.3, 20×15 MBs = **320×240** coded, no crop, poc_type 0, 4 refs; stts says exactly **45 samples** (delta 512, 25 fps); stss says 1 sync sample; and a full mdat scan of all 46 video NALs decoded slice_type per slice gives **1 I + 29 P + 15 B** — B-frames genuinely present (15 of 45 frames). This is the MP4-form half of lane C of `h264_sw_differential.rs` (pinned ffmpeg yuv420p MD5 `532c57170f56fcf4d20243b1242b88d3`), which passed 17/17 in the baseline.

## 2. What actually works (verified)
— python3: stsd→avc1 320x240; avcC profile 77 level 13 lengthSize 4, numSPS 1 (24 B), numPPS 1.
— python3 SPS decode: `profile 77 cons 0x40 level 13 ... w_mbs-1 19 h_map-1 14 ... crop_flag 0` → 320x240.
— python3 stts: `run: count 45 delta 512` → 45 frames; stss: 1 IDR.
— python3 mdat scan: `NAL types {6:1, 5:1, 1:44}`, `slice_types {7:1, 5:15, 6:29}` → 1 I + 15 B + 29 P (slice_type 0/5 = B).
— Baseline: `lane_c_bframes_mp4 ... ok` (audit/runs/baseline/cargo-test-workspace.txt:1173) and `lane_c_bframes_annexb ... ok` (1172) inside the 17/17, 24.74s run.

## 3. Findings
None. Checked: box tree integrity, avcC consistency with the SPS it carries, sample count vs name, B-frame presence vs name (slice-type scan), single-IDR structure, framing lengths all within mdat.

## 4. Per-function table
N/A (binary asset).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| h264_sw_differential lane_c_bframes_mp4 | YES | pinned whole-stream yuv420p MD5 vs ffmpeg 7.1.5 through the MP4/avcC/length-prefixed path — the framing-switch lane; ran in baseline |
| lane_c_bframes_annexb (sibling .h264 fixture) | YES | same MD5 through the Annex-B path |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "six lanes × both entry forms, bit-exact vs ffmpeg 7.1.5" | crates/codecs/src/video/h264/mod.rs:20-23 | REAL | baseline 17/17 differential pass with pinned MD5s (this file = lane C MP4 form); ffmpeg not re-run here (UNVERIFIED directly) |

## 7. Dead / duplicated / unreachable code
N/A.

## 8. Dependencies & flags
Data-only asset (55,978 bytes matches manifest row).

## 9. Open questions for the next agent
None.
