# CLAIM AUDIT: docs/formats/video-mjpeg.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R5) |
| Claims extracted | 9 |
| REAL 7 / REAL-UNPROVEN 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The strongest codec record in the batch and the tree proves it: the in-house claim is real
(zero jpeg-decoder/zune-jpeg deps; 10-module hand-built mjpeg crate), the matrix row says
implemented ×3, and the orchestrator baseline EXECUTED the claim end-to-end —
rc-smoke headless-decode PASS (v_decoded==v_frames) on the AVI/MJPEG/PCM fixture and
engine-info "decoders: mjpeg, pcm_s16le". The AVI1 interlace work (5.H3) is present in
module docs with the ffmpeg-doesn't-parse-AVI1 statement; the 17-lane differential file
exists with exactly 3/3 tests as the record states. Differential magnitudes are at-time.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | status = implemented (baseline sequential DCT, AVI1.1, 4:2:0/4:2:2/4:4:4) | :10-11 | REAL | feature-matrix.toml:541-549 `codec-video-mjpeg` status windows/linux/macos = implemented, web not_started; ORCHESTRATOR-BASLINE: audit/runs/baseline/rc-smoke.txt:129-131 "headless-decode (decode-bench; every frame must decode) ... PASS (v_decoded==v_frames, a_decoded==a_frames)" + :128 "decoders: mjpeg, pcm_s16le". |
| 2 | crates = [] — in-house, zero dependencies (D-018; NOT jpeg-decoder/zune-jpeg) | :15-17 | REAL | `rg -c "jpeg-decoder|zune-jpeg" crates/codecs/Cargo.toml` -> 0 matches; ls crates/codecs/src/video/mjpeg/ -> bitreader color decoder error field fuzz huffman idct idct_simd mod parser (10 modules, superset of the doc's list). |
| 3 | AVI1.1: APP0 AVI1\0 + polarity byte, vocab 0/1/2, typed structural errors; ffmpeg neither writes nor parses AVI1 | :31-37 | REAL | mod.rs:41-62 "Field-based MJPEG / AVI1 (card 5.H3) ... APP0 segment carrying `AVI1\0` ... ffmpeg neither writes nor parses AVI1 (no `AVI1` byte sequence" (sed read). |
| 4 | Typed-Unsupported: progressive (SOF2)/12-bit/arithmetic/etc. | :38-40 | REAL | error.rs:167 `unsupported("progressive DCT (SOF2)")`, :203 `unsupported("12-bit sample precision")` + test :211. |
| 5 | fuzz_target = fuzz_codec_mjpeg, survival suite green | :13-14 | REAL | crates/codecs/src/video/mjpeg/fuzz.rs + fuzz_codec_mjpeg refs in mod.rs (rg --files-with-matches); CI wiring absent as admitted. |
| 6 | 17-lane differential pinned in tests/mjpeg_field_differential.rs; 3/3 tests | :59-60/:70-71 | REAL | crates/codecs/tests/mjpeg_field_differential.rs exists; `rg -c "#\[test\]"` -> 3 — exactly the "3/3" the record states. |
| 7 | Tests at gates: 2.B4 workspace 881/0/3; 5.H3 codecs 371/0 | :70-71 | REAL-UNPROVEN | at-time gate runs; current mjpeg src census = 91 #[test] blocks (suite has grown); no cheap re-run (load discipline) — baseline cargo-test-workspace.txt: codecs packages exit 0. |
| 8 | 2.B4 oracle cross-check: 120/120 frames hash-match; luma byte-exact under both ffmpeg IDCTs; chroma max Δ1 on 1.068% | :45-52 | REAL-UNPROVEN | measured at evidence time; committed reference digests are in-tree (tests/data reference files); not re-executed here. |
| 9 | 5.H3 differential: 0015/0125 bit-exact; other lanes ±1 LSB; 444→RGBA max 3; 422→RGBA max 4 (43.4% on 0064, recorded convention divergence) | :53-58 | REAL-UNPROVEN | at-time re-measurement; the honest-gaps section (:75-78) records the box-vs-fancy upsampling convention divergence the numbers express — self-consistent. |

## 3. Contradictions
None. The record discloses its own tooling failure history (:61-64: earlier sweeps reported
max-255 garbage before the script bug was found) — the opposite of cherry-picking.

## 4. Fabrication / inflation indicators
None — the "honest gaps" list (fancy upsampling, colorimetry default, web column) matches
the code's typed refusals and the matrix's web=not_started.

Commands run: rg codec-video-mjpeg + status in matrix; ls mjpeg module; rg -c jpeg-decoder|zune-jpeg Cargo.toml (0); ls codecs/tests (mjpeg_field_differential.rs); rg -c "#\[test\]" mjpeg src (91) + differential (3); rg AVI1/SOF2/12-bit in mjpeg; rg fuzz_codec_mjpeg; baseline rc-smoke.txt:128-131 + mp4-smoke.md read.
