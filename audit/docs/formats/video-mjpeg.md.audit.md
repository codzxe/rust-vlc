# CLAIM AUDIT: docs/formats/video-mjpeg.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 8 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 REAL / 2 REAL-UNPROVEN (differential numbers not re-executed) / 0 / 0 / 0 / 0 / 1 (ffmpeg-internal "AVI1 absent" claim) |

## 1. Blunt verdict
The strongest doc audited so far: in-house mjpeg codec exists (6 modules, verified files),
matrix row honestly says `implemented` with evidence that names its own limits (Linux-only,
mock-labeled CI, web not_started). Differential numbers are pinned in a real 3-test file and
the decode itself is proven by the orchestrator baseline (headless AVI/MJPEG decode exit 0).
No fabrication found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "matrix row already `implemented` on win/linux/macos with 2.B4 evidence" | L7-9 | REAL | feature-matrix.toml:549-551 status implemented×3 (web not_started), evidence names 2.B4; note admits "ALL evidence to date is Linux-only, mock-labeled, CI 3-OS PENDING" |
| 2 | "in-house, zero dependencies (D-018; NOT jpeg-decoder/zune-jpeg)" | L15-17 | REAL | crates/codecs/Cargo.toml [dependencies] = codec-api + media-types only; rg jpeg-decoder/zune → no hits |
| 3 | "self_built = [`codecs::video::mjpeg` … decoder/parser/mod/field/idct_simd" | L18-19 | REAL | ls crates/codecs/src/video/mjpeg/ → decoder.rs parser.rs mod.rs field.rs idct_simd.rs (+bitreader/color/error/huffman/idct/fuzz) |
| 4 | "AVI1.1 … APP0 `AVI1\0` + polarity byte, vocab 0/1/2 … typed structural errors" | L31-34 | REAL | mjpeg/mod.rs:41-62 documents+implements AVI1 paths (module header, verified by rg) |
| 5 | "17-lane table pinned in `tests/mjpeg_field_differential.rs`" | L59-60 | REAL (file real) | crates/codecs/tests/mjpeg_field_differential.rs exists; lanes 0015/0125/0064 named at :110/:174/:198; exactly 3 `#[test]` (rg count) = doc's "3/3" |
| 6 | "fuzz_target = `fuzz_codec_mjpeg` (registered)" | L13 | REAL | fuzz/fuzz_targets/fuzz_codec_mjpeg.rs exists |
| 7 | "2.B4 oracle cross-check … 120/120 frames … hash-match" | L45-47 | REAL-UNPROVEN | evidence ledger cited by matrix row exists? not re-executed (cargo-build-budget; corpus-v4 media external). Baseline proves decode: rc-smoke headless-decode AVI/MJPEG/PCM PASS, engine-info decoders mjpeg, pcm_s16le |
| 8 | "5.H3 differential … 0015/0125 bit-exact … 444→RGBA max 3 … tests at gates codecs 371/0" | L53-71 | REAL-UNPROVEN | numbers pinned in the 3-test file (read); workspace baseline: 4744 passed/2 failed/51 ignored (baseline cargo-test-workspace.txt) — per-crate 371/0 not re-run; NOT executed here (cargo-build-budget). ffmpeg 7.1.5 present in sandbox (`ffmpeg -version` → 7.1.5-0+deb13u1) so the oracle-tool version claim is REAL |

## 3. Contradictions
None. Doc's own "Honest gaps" (L73-82) match matrix notes (web Phase-7 assessment, box
upsampling convention). Matrix note "ALL evidence … Linux-only, mock-labeled" is more
pessimistic than this doc's "implemented" framing — the pessimism is the truth-bearing side.

## 4. Fabrication / inflation indicators
None. External claim "ffmpeg 7.1 neither writes nor parses AVI1 (verified absent from
libavcodec)" (L36-37) UNVERIFIED — would require a libavcodec source check; environment
ffmpeg is the right version but absence-in-source not testable cheaply. LOC claim
"3 124 src + 1 423 test LOC at 2.B4" — current 5-module subset is 3,913 lines (grew with
5.H3 field work; consistent with "at 2.B4" qualifier).
