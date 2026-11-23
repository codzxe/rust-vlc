# AUDIT: crates/codecs/src/video/mjpeg/idct.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 558 (277 source + 281 tests) / 5 non-test fns + 10 tests |
| File verdict | REAL (independently re-derived in Python: basis table exact, factored transform exact, ±2 LSB of the true float IDCT on realistic blocks) |
| Reachable from shipped entry point? | YES — apps/cli pipeline_host "mjpeg" → MjpegDecoder/decode_image (mjpeg is a shipped decoder per rc-smoke engine-info) → decoder.rs:641 `H::transform` → decoder.rs:262 `idct_block_dispatch` → idct_simd.rs:141 fallback/default `super::idct::idct_block`; `idct8`/`shr32_round` also shared verbatim by the SSE4.1 kernel (idct_simd.rs:83,296-298) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 independent verification (below, 3 scripts); Read of full file; `rg` caller map; baseline citation audit/runs/baseline/cargo-test-workspace.txt:806-851 |

## 1. One-paragraph verdict
This IDCT is correct, and I proved it independently rather than trusting the suite: (1) the Q16 basis table matches `round(c(u)/2·cos((2x+1)uπ/16)·2^16)` for all 64 entries with zero mismatches; (2) a Python replication of the factored `idct8` (copied from the source's even/odd symmetry split) equals the naive 8×8 dot products on 20,000 random vectors with zero mismatches, so the ED-FIX-1 "pure reordering, bit-exact" claim is true; (3) the full two-pass fixed-point block transform lands within ±2 LSB of the true float JPEG IDCT over 2,000 random dequantized blocks (±15 only on hostile ±i16·Q255 inputs the module never claims accuracy for, only boundedness); (4) the DC-only test values (133/123/132) reproduce exactly through 23170²/2³² = 0.125 rounding. The overflow analysis in the module doc checks out arithmetically (max pass-2 |acc| ≈ 7·10¹⁶ < 2⁶³, clamp at ±2²⁰ makes it input-independent). The only defect found is a misleading comment about who uses the short-slice prefix write. This is the strongest file audited so far.

## 2. What actually works (verified)
— python3: table-vs-formula mismatches: 0 (all 64 A[u][x] = round(c(u)/2·cos((2x+1)uπ/16)·2^16), c(0)=1/√2).
— python3: factored-vs-naive mismatches over 20,000 random w vectors: 0 (factored `idct8` re-implemented from source lines 124-190 vs naive Σ_v A[v][x]·w[v]).
— python3: worst |fixed-point vs true float IDCT| = 2 over 2,000 random dequantized blocks (realistic |q|≤255, Q≤255); = 15 only for hostile i16×Q255 blocks (bounded, no overflow, clips to 0/255 as the test asserts).
— python3: DC=40 → 133, DC=−40 → 123, DC=17·Q2 → 132 — all three test expectations reproduce.
— Baseline: `idct::tests::*` 10/10 ok incl. `factored_idct_is_bit_exact_vs_naive` (8192 random blocks × 64 random quant tables in-process) and `hostile_coefficients_cannot_overflow` (baseline:806-851).

## 3. Findings
F-001 [S4]
Where: idct.rs:238-241
Code: (verbatim)
```
    // Prefix write (the decoder writes into compact planes; a short
    // slice must not panic and fills what fits).
    let n = out.len().min(64);
    out[..n].copy_from_slice(&samples[..n]);
```
What is wrong: The comment's justification is false — the production decoder never passes a short slice. decoder.rs:639-641 always transforms into a full `let mut samples = [0u8; 64]` and lets `Plane::write_block` (decoder.rs:268-270) crop geometry. The only callers of the prefix path are idct_simd.rs's equivalence tests (`&mut s2[..out_len]`, idct_simd.rs:354-355) and this file's own `short_output_slices_are_written_prefix_only`. Also note the prefix semantics are NOT "w·h row-major samples" for w<8 (a 5×5 block would need strided samples, not the first 25) — harmless today because nobody depends on that reading, but the doc invites it.
Confidence: HIGH — Verification: read decoder.rs:633-641 + idct_simd.rs:354-355 caller map.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| shr32_round | 105-111 | REAL | — | round-half-away; shared with SIMD kernel; no i64::MIN path reachable (bounds proven) |
| idct8 | 124-190 | REAL | — | verified exact vs naive (20k vectors); zero-term skips are exact no-ops |
| idct_block | 197-242 | REAL | — | DC fast path verified (23170²/2³²=0.12499863); dequant clamp; prefix write (F-001) |
| idct_block_naive (cfg(test)) | 248-276 | REAL | — | honest oracle, structurally independent of the factored form |
| TestRng | 450-461 | REAL | — | xorshift64*, deterministic |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| zero_block / pure_dc / negative_dc / dc_saturation / quantization_scales_the_dc | YES | exact constants, independently reproduced in python |
| horizontal_ramp_highest_frequency | YES | antisymmetry + row-equality invariants |
| hostile_coefficients_cannot_overflow | YES | asserts saturation to 0/255 (would catch overflow panics in debug) |
| short_output_slices_are_written_prefix_only | YES | prefix fill asserted |
| round_trips_against_a_reference_forward_dct | YES | documented ±8 bound on FDCT-roundtrip; would fail on algebra errors |
| factored_idct_is_bit_exact_vs_naive | YES | 8192 random blocks vs structurally independent naive oracle + structured sweeps; the strongest test in the file |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the factored form is a pure algebraic reordering ... bit-exact vs the naive form" | idct.rs:33-35 | REAL | python 20k-vector equivalence + in-process 8192-block baseline test |
| "the same Q16 table the in-repo 2.A3 encoder uses" | idct.rs:4-5 | REAL-UNPROVEN | encoder table not independently diffed (tools/xtask fixtures.rs not re-read); UNVERIFIED but the table itself is formula-exact |
| "the factored intermediates are bounded by ... < 2⁶³" | idct.rs:52-56 | REAL | re-derived: 4·32138·2²⁰ ≈ 1.35e11, 8·32138·2.7e11 ≈ 7e16 < 9.2e18 ✓ |
| "±1 LSB" real-file divergence class vs ffmpeg | mjpeg_field_differential.rs:26-28 (downstream claim) | REAL-UNPROVEN here | my random-block worst is ±2; the real-file ±1 claim rests on the ffmpeg-pinned MD5 lanes in that test (next file) |
| "the decoder writes into compact planes" | idct.rs:238 | FALSE | decoder passes full 64-byte buffers (F-001) |

## 7. Dead / duplicated / unreachable code
None. `DCT_A` is cfg(test) since ED-FIX-1 (documented, kept as the oracle — legitimate). The prefix path is test-only but not dead (idct_simd equivalence uses it).

## 8. Dependencies & flags
No external deps, no features. Consumers: idct_simd.rs (dispatch + SSE4.1 kernel reusing idct8/shr32_round verbatim — bit-exactness by shared code), decoder.rs via dispatch. The SSE4.1 path is an unsafe block in idct_simd.rs — audited by whoever owns that file; the sharing design limits its divergence surface.

## 9. Open questions for the next agent
1. idct_simd.rs auditor: the `unsafe fn idct_block_sse41` (idct_simd.rs:212) must prove its i16×u16→i32 widening and accumulator bounds stay under the same ±2²⁰ clamp math; the scalar fallback (line 218) exists for that.
2. mjpeg_field_differential.rs (my next file) carries the external ffmpeg evidence for this transform — check its lane honesty.
