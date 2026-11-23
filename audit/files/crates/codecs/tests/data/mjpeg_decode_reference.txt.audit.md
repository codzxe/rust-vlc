# AUDIT: crates/codecs/tests/data/mjpeg_decode_reference.txt (reference data)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | ASCII text, 60 frame digests (sha256 per frame, 115200 B I420 each) + provenance header / 5681 bytes / 52305fe9760324fe90a9cf77d4da75538e1ec743a5a9d3c16769e6ffb9d0d940 |
| File verdict | REAL (live golden evidence, consumed by executed tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -c '^[0-9a-f]{64}$'` -> 60 digests; `./target/debug/deps/fixtures_parent-4e25a1e1316c527e` -> 9 passed / 0 failed, 0.51s (incl. `decoded_avi_outputs_match_reference_digests`, `decoded_mp4_outputs_match_reference_digests`, `sha256_reader_pinned_against_committed_file_digests`) |

## 1. Verdict
Live, load-bearing golden file: 60 per-frame sha256 digests of the MJPEG decoder's I420 output for the 60-frame corpus shared byte-identically by the two class-A parent fixtures (AVI + MP4). The consuming suite extracts the frames, decodes through the real codec-api `MjpegDecoder` path, and hash-matches every frame against this file — both container lanes ran green when I executed them. The provenance header is unusually honest: it states the digests are the project's own decoder output (self-generated goldens — the usual red flag) but records the ffmpeg 7.1.5 oracle cross-check with exact numbers (luma byte-exact 0/4,608,000 differing under both ffmpeg IDCT variants; chroma max |diff| = 1 on 1.068%/0.365 of samples, confined to 12 of 300 chroma blocks per frame).

## 2. Test usage (verified by execution)
`./target/debug/deps/fixtures_parent-4e25a1e1316c527e` -> **9/9 pass** in 0.51s: `decoded_avi_outputs_match_reference_digests` + `decoded_mp4_outputs_match_reference_digests` (this file's 60 digests, both containers), `avi/mp4_extraction_matches_writer_declared_frame_digests`, `both_containers_carry_the_identical_frame_essence`, plus damage/recovery lanes. The suite also pins its own sha256 reader against this committed file's digest (`sha256_reader_pinned_against_committed_file_digests`).

## 3. Findings
F-001 [S4]
Where: mjpeg_decode_reference.txt:11-20 (provenance header)
Code:
# PROVENANCE (honesty rules, doc 03 sec.7 / AGENTS.md sec.3.4): the digests are
# Lumen's own decoder output, cross-checked against the ffmpeg 7.1.5 oracle
What is wrong: the goldens are self-generated (decoder-under-test output). The ffmpeg oracle cross-check is recorded as a one-time measurement in the header, not re-executed by any test — so the suite pins current behavior, not oracle-correctness: a future decoder change that shifts chroma rounding further from ffmpeg (but consistently) fails the digest test as a "regression" with no way to tell direction, and a wholesale regeneration of this file would silently bless whatever the decoder then produces. The header's candor and the exact divergence numbers mitigate this; the pattern is the standard self-golden trade-off, documented. Confidence: HIGH — Verification: read header + ran consumers.

## 4. Matrix evidence
This file is the digest backbone of the 2.B4 first-vertical decode claim (the mjpeg lane the rc-smoke headless-decode exercises). Frame count (60) matches the parent fixtures' declared streams; all consumers ran green.
