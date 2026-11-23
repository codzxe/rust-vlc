# AUDIT: crates/codecs/tests/fixtures/vp9/corpus_reenc_vp8to9_480x360_41f.ivf
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 10072 (14 parts) / binary asset |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; consumed by crates/codecs/tests/vp9_differential.rs lane D via `fixture_path`/`ivf_packets`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file; stat; python3 struct walk; ffprobe -count_packets (rc 0); ffmpeg libvpx-vp9 decode -> md5sum (rc 0) |

## 1. One-paragraph verdict
Genuine VP9 bitstream, not a fabricated fixture. `file` identifies "Duck IVF video file, codec VP90, 480x360, 41 frames" (369573 bytes). My own struct walk confirms a well-formed DKIF header (version 0, hdrlen 32, fourcc VP90, 480x360, rate 25/scale 1, header frame count 41) and 41 complete frames with zero truncation and zero trailing bytes, timestamps 0..40 strictly monotonic, first packet 128781 bytes (keyframe). The independent ffmpeg 7.1.5 `libvpx-vp9` oracle decodes it cleanly and the yuv420p dump MD5 equals the constant pinned in vp9_differential.rs:88 — so the pinned truth was not invented by the code under test.

## 2. What actually works (verified)
- `file` -> "Duck IVF video file, codec VP90..., 480x360, 41 frames".
- python3 struct walk -> 41/41 frames, no truncation/trailing, ts 0..40 monotonic.
- `ffmpeg -c:v libvpx-vp9 ... -pix_fmt yuv420p - | md5sum` -> `9621dcf981bfc3b52efa40c66c66a543` == pinned `yuv_md5` at vp9_differential.rs:88 (verified with `sed -n '84,89p' crates/codecs/tests/vp9_differential.rs`).
- `ffprobe -count_packets` -> codec_name=vp9, 480x360, nb_read_packets=41 (== lane's `frames: 41`).
- Baseline run (audit/runs/baseline/cargo-test-workspace.txt:1384-1393): `differential_lanes_vs_ffmpeg_libvpx_vp9 ... ok`, suite 4/4 — this fixture was demuxed+decoded through the wired path at baseline time.

## 3. Findings
None. Checked and how: header fields vs IVF spec (python3); frame table integrity (walk); monotonic timestamps; oracle decodability (ffmpeg); MD5 vs pinned constant (md5sum); frame-count claim vs ffprobe; citation in vp9_differential.rs:85-89 (sed-verified); no other repo doc cites this file by name except audit/waves manifest/assignments (rg).

## 4. Per-function table
N/A (binary asset).

## 5. Test honesty
The consuming test `differential_lanes_vs_ffmpeg_libvpx_vp9` (vp9_differential.rs:246-335) would fail if the decoder's pixel output diverged (whole-stream MD5 == pinned/record constant, plus live-oracle per-frame identity when ffmpeg is on PATH). Not a tautology: the pinned MD5 independently reproduces from the fixture via ffmpeg in this sandbox.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| lane D: 41 frames, 480x360, yuv_md5 9621dcf9... | vp9_differential.rs:84-89 | REAL | ffprobe count=41; ffmpeg dump MD5 matches exactly |
| "vp9_differential 4/4 pass in the codecs suite" | docs/feature-matrix.toml:481 | REAL | baseline cargo-test-workspace.txt:1384-1393 (4 passed) |

## 7. Dead / duplicated / unreachable code
N/A. Note for the next agent: docs/worklog/archive-2026-10.md:10660 claims "vp9_differential 6/0" for an earlier era; the current suite has 4 tests — 2 tests were dropped at some point (doc-history question, not a fixture defect).

## 8. Dependencies & flags
None (test data). Consumed only under `crates/codecs` integration tests; not shipped in any binary.

## 9. Open questions for the next agent
Whether the removed 2 of the formerly-6 vp9_differential tests reduced coverage (check git history is orchestrator-side; UNVERIFIED here).
