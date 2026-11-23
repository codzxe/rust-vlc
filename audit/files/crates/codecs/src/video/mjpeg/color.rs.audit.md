# AUDIT: crates/codecs/src/video/mjpeg/color.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 860 / 22 prod + 10 tests/helpers |
| File verdict | REAL (conversion math independently verified against a python ITU-T T.871 reference; hot/cold equivalence pinned by tests and by ffmpeg-digested RGBA lanes) |
| Reachable from shipped entry point? | YES — AVI: crates/demux/src/avi/header.rs:219 `StreamFormat::MotionJpeg { .. } => "mjpeg"` → apps/cli/src/pipeline_host.rs:242 `"mjpeg"` → decoder.rs:961 `OutputFormat::Rgba8 => assemble_rgba_into` → decoder.rs:1174/1189 `RgbComposer::compose_row` (for any non-4:2:0, non-grayscale multi-component JPEG) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 reference cross-check (below); rg/sed reachability + quote verify; baseline cites |

## 1. One-paragraph verdict
The YCbCr→RGB conversion is correct. I reimplemented the math in python (exact ITU-T T.871 reals with round-half-away, the same rounding this code uses) and compared: all six committed hand-computed vectors match exactly — (90,110,200)→(191,45,58), (30,220,120)→(19,4,193), (255,255,255)→(255,121,255), (0,0,0)→(0,135,0), (200,90,240)→(255,133,133), (10,240,110)→(0,0,208) — and a 3.8M-sample sweep plus a full 256³ G-channel enumeration shows the fixed-point code deviates from exact T.871 on only 9096/16,777,216 triples (0.054%), always by exactly 1 LSB at fixed-point rounding boundaries (the same accuracy class as libjpeg's integer path). Box (nearest-neighbor) chroma upsampling is implemented as documented; the row-hoisted hot path is pinned byte-identical to the per-sample reference by a randomized sweep over all four sampling geometries (test at lines 696-784, baseline ok). Remaining issues are hygiene: three dead/future seam functions and a production-dead grayscale path.

## 2. What actually works (verified)
- python3 reference cross-check (command + output above): 6/6 committed vectors exact; max channel delta vs exact T.871 = 1 LSB on 0.054% of the input space.
- Baseline cargo-test-workspace.txt:763-772: all 10 color unit tests ok, including `compose_row_matches_the_per_pixel_reference` (randomized hot-vs-cold oracle) and `branchless_round_matches_the_reference_everywhere`.
- Baseline :1216-1223: `mjpeg_field_differential` 3/3 ok with `DecodedFormat::Rgba8` lanes (4:2:2 odd-size, 4:4:4 corpus) — the compose path runs against ffmpeg-pinned digests.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/video/mjpeg/color.rs:70-75
Code:
```
const CR_R: i32 = 91881; // 1.402
const CB_G: i32 = -22554; // -0.344136
const CR_G: i32 = -46802; // -0.714136
const CB_B: i32 = 116127; // 1.772
```
What is wrong: CB_B = 116127 is not the nearest Q16 of 1.772 (1.772 × 65536 = 116129.8; libjpeg's FIX(1.772) = 116130). The other three constants ARE nearest-Q16. The deviation (3/65536) contributes part of the measured 9096 ±1-LSB G/B deviations vs exact T.871 (my python full-space enumeration; the rest is inherent Q16 accumulation on rounding boundaries). Sub-visible, libjpeg-class accuracy, but the comment "the classic JFIF equations / rounded at authoring time" is inaccurate for this one constant. Confidence: HIGH — Verification: ran python reference (full 256³ G-channel enumeration: 9096 diffs, max delta 1).

F-002 [S4]
Where: crates/codecs/src/video/mjpeg/color.rs:351-362, 476-479
Code:
```
    #[allow(dead_code)] // baseline commit only: unused until the W6-03 rewrite arms it
    pub(crate) fn compose_row_simd(&self, row_y: usize, width: usize, out: &mut [u8]) {
        self.compose_row(row_y, width, out);
    }
```
What is wrong: three dead seam functions (`compose_row_scalar`, `compose_row_simd`, `simd_available`→`false`). `compose_row_simd` is byte-for-byte the scalar path — the name is false today (honestly documented as a W6-03 future seam, and examples/color_bench.rs:36-39 says the simd lane is "absent from the baseline phase binary"). Any current run of color_bench's `simd` lane benchmarks the scalar kernel. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/codecs/src/video/mjpeg/color.rs:336-342, 412-426
Code:
```
            (None, Some(luma)) => {
                if out.len() >= width * 4 {
                    self.compose_gray_row(luma, width, out);
```
What is wrong: the grayscale arm of `compose_row` / `compose_gray_row` / `compose_gray_direct` is production-dead: the only caller (decoder.rs:1145-1193 `assemble_rgba_into`) passes `chroma: None` only when `planes.len() == 1`, but a 1-component frame takes `OutputFormat::I420` (decoder.rs:1046-1049 `if info.components.len() == 1 { return OutputFormat::I420; }`), so `assemble_rgba_into` never runs with `chroma == None`. Only unit tests exercise the gray path. Belt-and-braces for future callers, but as shipped it is dead code. Confidence: HIGH — Verification: read both call sites + component-count gate (decoder.rs:552-559 refuses 2/4-component frames).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ycc_to_rgb | 78-90 | REAL | — | reference math; python-verified |
| shr16_round | 93-99 | REAL | — | round-half-away, branch form |
| shr16_round_b | 109-112 | REAL | — | branchless; equality swept in tests (763) |
| ycc_to_rgb_fast | 117-129 | REAL | — | triple-for-triple equal to reference (baseline :766) |
| RowSampler::new | 175-194 | REAL | — | checked row bounds; None → cold path |
| RowSampler::sample/step | 199-212 | REAL | — | Bresenham stepper, exact floor |
| identity_row / half_row | 219-238 | REAL | — | direct-row proofs (valid_w/row.len guards) |
| compose_rows_1to1 | 244-256 | REAL | — | 4:4:4/4:4:0 fast path |
| compose_rows_2to1 | 260-274 | REAL | — | 4:2:2 fast path; chroma `x >> 1` |
| compose_gray_direct | 278-283 | REAL (dead in prod) | S4 | F-003 |
| RgbComposer::new | 300-314 | REAL | — | |
| RgbComposer::compose_row | 318-345 | REAL | — | dispatch + out.len guard |
| compose_row_scalar / compose_row_simd | 352-362 | DEAD (seam) | S4 | F-002 |
| compose_color_row | 371-409 | REAL | — | direct-row then stepper fallback |
| compose_gray_row | 412-426 | REAL (dead in prod) | S4 | F-003 |
| compose_row_cold | 434-459 | REAL | — | historical reference behavior, prefix-safe writes |
| sample_up | 463-471 | REAL | — | neutral-128 for out-of-data |
| simd_available | 477-479 | DEAD (seam) | S4 | always false |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| branchless_round_matches_the_reference_everywhere | YES | negative-tie sweep + dense window |
| fast_conversion_matches_the_reference_everywhere | YES | all 256×256 (cb,cr) × clamp-boundary y values |
| neutral_chroma_is_identity | YES | R=G=B=Y for 7 y values |
| known_triples_convert_correctly | YES | exact vectors (python re-verified by me) |
| out_of_gamut_triples_clamp | YES | exact clamp vectors |
| grayscale_row_passes_through_luma | YES | but tests a production-dead path (F-003) |
| chroma_upsample_box_replication | YES | exact per-pixel expectations |
| compose_row_matches_the_per_pixel_reference | YES | randomized xorshift sweep, 4 geometries × odd/even widths × 5 rows, hot vs reference oracle |
| inconsistent_plane_geometry_keeps_the_neutral_reads | YES | short plane → neutral 128, no panic |
| short_output_rows_stay_prefix_safe | YES | 1.5-pixel output buffer |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ITU-R BT.601 full-range YCbCr ↔ RGB, the JFIF conversion, fixed point" | color.rs:1-3 | REAL | python T.871 reference: exact on all committed vectors, ±1 LSB on 0.054% |
| "Upsampling is nearest-neighbor (box) replication" | color.rs:9-12 | REAL | RowSampler/stepper code + box_replication test |
| "output is bit-identical to the previous implementation" | color.rs:40-42 | REAL | compose_row_matches_the_per_pixel_reference (baseline ok) |
| "constants ... the classic JFIF equations" | color.rs:70-71 | PARTLY FALSE | CB_B=116127 vs 1.772→116130 (F-001) |
| simd entry "absent from the baseline phase binary — explicit, not silent" | examples/color_bench.rs:36-39 | TRUE (honest) | simd_available()→false |

## 7. Dead / duplicated / unreachable code
- F-002 seam trio; F-003 gray path (production-dead, test-only).
- examples/color_bench.rs embeds a ~160-line verbatim "legacy" copy of this kernel (lines 183+) — deliberate bench isolation, documented, but a drift hazard (S4, folded into F-002).

## 8. Dependencies & flags
None: pure safe Rust, no crates, no features. The `unsafe_code=deny` workspace lint is untouched here.

## 9. Open questions for the next agent
- decoder.rs (mjpeg) is another agent's file: the 2-/4-component refusal (decoder.rs:552-559) is what keeps `planes[2]` indexing in `assemble_rgba_into` panic-free; if that gate ever loosens (Adobe CMYK), assemble_rgba_into panics on planes[2] — worth a cross-file note.
- W6-03 SIMD kernel: does it exist on any branch? Until it lands, color_bench `simd` numbers are scalar numbers.
