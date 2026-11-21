# AUDIT: crates/codecs/src/video/mjpeg/idct.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 558 / 16 items (5 prod consts/fns + 3 test-only items + TestRng + 10 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI open AVI → apps/cli/src/pipeline_host.rs "mjpeg" → MjpegDecoder::decode_scan (decoder.rs:635 `H::transform(&block, qt, &mut samples)`) → idct_simd.rs:141 `super::idct::idct_block` — the scalar DEFAULT arm; executed in baseline `decode-bench` 60/60 frames) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg "idct_block\|idct8\|shr32_round" (callers: idct_simd.rs:141,218,339-355,449 — dispatch + its equivalence tests); rg "transform\(" decoder.rs:635 (production call site, out slice is exactly `[0u8; 64]`); rg "idct::tests::" baseline log → 10/10 ok (:709-718); independent algebra re-derivation of all constants and the factoring (below); rg "sha256" tests/fixtures_parent.rs (digest claim real) |

## 1. One-paragraph verdict
This is the fixed-point 8×8 inverse DCT of the shipped MJPEG first-vertical lane, and it is the rare file where I re-derived the mathematics independently and found it correct. The Q16 basis constants (23170/32138/27246/18205/6393/30274/12540 rows) match `c(u)/2·cos((2x+1)uπ/16)` to the last rounding digit; the factored even/odd split is a provably exact reordering (even rows of the basis are symmetric in x↔7−x, odd rows antisymmetric — both verified element-wise against the DCT_A oracle table); the overflow bound analysis (|pass-2 out| ≤ 8·32138·2.7·10¹¹ ≈ 7·10¹⁶ < 2⁶³) is arithmetically right; and the ±2²⁰ coefficient clamp is invisible on legitimate data (max legit |q·Q| = 2047·255 = 521,985 < 1,048,576). All 10 unit tests ran green in baseline, including an 8192-block randomized bit-exactness sweep against the naive reference. The only defects are two contract-level nits with no production trigger.

## 2. What actually works (verified)
— `rg -n "idct::tests::" audit/runs/baseline/test-workspace.log.txt` → all 10 tests `ok` (:709-718), incl. `factored_idct_is_bit_exact_vs_naive` and `hostile_coefficients_cannot_overflow`.
— Baseline end-to-end: `decode-bench` on `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → `v_frames=60 v_decoded=60` (INDEX.md:78) — this file's transform executed on every decoded frame of the shipped lane.
— Independent verification (auditor's own algebra, this session): DC fast path `shr32_round(w0·23170²)/2³² + 128` equals the naive two-pass product for zero-AC blocks (23170²·w0 appears identically in both); spot checks: DC=40,Q=1 → 133, DC=−40 → 123, q=17,Q=2 → 4.25→4 → 132 — all three match the committed test expectations.
— `cargo xtask fixtures --check` PASS in baseline (INDEX.md row 12) — the end-to-end digest pinning (`tests/fixtures_parent.rs` per-frame sha256) that transitively covers this transform is live.

## 3. Findings

### F-001 [S4] Fast path and slow path disagree on `out.len() > 64`; contract documented only for `≤ 64`
Where: crates/codecs/src/video/mjpeg/idct.rs:197-242
Code:
```
pub(crate) fn idct_block(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
    // Zero-AC fast path: ...
    if block[1..].iter().all(|&q| q == 0) {
        ...
        out.fill(sample);
        return;
    }
```
```
    // Prefix write (the decoder writes into compact planes; a short
    // slice must not panic and fills what fits).
    let n = out.len().min(64);
    out[..n].copy_from_slice(&samples[..n]);
```
What is wrong: For an `out` slice longer than 64 the zero-AC fast path fills the ENTIRE slice while the general path writes only the first 64 bytes and leaves the tail untouched. The doc comment (":196 `out` receives `w*h ≤ 64` row-major samples") makes >64 a contract violation, but nothing asserts it, and the two arms of the same function then disagree about what a violation means.
Why it matters: Zero today — the only production caller passes exactly 64 (`decoder.rs:634-635` `let mut samples = [0u8; 64]; H::transform(&block, qt, &mut samples);`) and the SIMD equivalence tests slice to `≤64`. It is a loaded footgun for the next caller (e.g., a whole-row writer) and an inconsistency a `debug_assert!(out.len() <= 64)` would have made impossible.
What correct looks like: `out[..out.len().min(64)].fill(sample)` in the fast path, plus `debug_assert!(out.len() <= 64)` at entry.
Confidence: HIGH — Verification: read; both arms traced; production call site read (decoder.rs:634-635).
### F-001a folded: `shr32_round` documents no domain restriction
Where: crates/codecs/src/video/mjpeg/idct.rs:105-111
Code:
```
pub(crate) fn shr32_round(value: i64) -> i64 {
    if value >= 0 {
        (value + (1i64 << 31)) >> 32
    } else {
        -((-value + (1i64 << 31)) >> 32)
    }
}
```
What is wrong: `value = i64::MIN` overflows `-value` (and `value + (1<<31)` at the negative extreme is fine, but the negation is not). No doc states the precondition (|value| ≤ ~7·10¹⁶ per the module bound, 13 orders below the limit).
Why it matters: The function is `pub(crate)` and shared with idct_simd.rs pass 2 — a future caller outside the proven bound gets a wrapped result silently. Cosmetic today because every in-crate call site is inside the module's proven |value| ≤ 7·10¹⁶ envelope.
What correct looks like: A doc line "callers must guarantee |value| < 2⁶² (see module bounds)" or `debug_assert!(value != i64::MIN)`.
Confidence: HIGH — Verification: read; all call sites (idct.rs:203,234,270; idct_simd.rs reuse) are inside the proven bound.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_COEFF` const | 64-65 | REAL | — | 2²⁰; proven invisible on legit data (521,985 < 1,048,576 — auditor-verified arithmetic). |
| `DCT_A` (cfg(test)) | 72-82 | REAL | — | Standard JPEG Q16 basis; auditor re-derived all 64 entries — every one matches `c(u)/2·cos((2x+1)uπ/16)` rounded. |
| consts A02/A12/A06/A16/A0 | 86-90 | REAL | — | Match DCT_A rows 2/6/0/4 magnitudes; verified. |
| consts O1/O3/O5/O7 | 94-97 | REAL | — | Match DCT_A rows 1/3/5/7 first 4 entries; verified element-wise. |
| `DC_SCALE` const | 100 | REAL | — | 23170² = 536,848,900; correct Q32 DC scale. |
| `shr32_round` | 105-111 | REAL | S4 | Round-half-away-from-zero; correct for the proven domain; i64::MIN precondition undocumented (F-001a). |
| `idct8` | 124-190 | REAL | — | Factored exact transform; even-part butterflies and odd-part zips re-derived algebraically — bit-exact reordering of Σ_v A[v][x]·w[v]; 22 MACs worst case; zero-group skips correct (`(w[0]\|w[4]) != 0`). |
| `idct_block` | 197-242 | REAL | S4 | DC fast path algebraically identical to naive (verified); dequant fold correct; pass-2 strided transpose read correct; normative clip at :234; fill/prefix asymmetry (F-001). Hot path of the shipped MJPEG lane (decoder.rs:635). |
| `idct_block_naive` (cfg(test)) | 248-276 | REAL | — | True naive double matmul; the oracle. Uses `out.get_mut` (bounds-safe prefix write). |
| test `zero_block_is_flat_mid_gray` | 284-290 | REAL | — | Ran ok (baseline :717). Would fail if level shift broke. |
| test `pure_dc_block_is_constant` | 292-302 | REAL | — | Ran ok (:714). Expected 133 re-derived by auditor: 40/8=5, 128+5. |
| test `negative_dc_shifts_down` | 304-311 | REAL | — | Ran ok (:713). Symmetry check. |
| test `dc_saturation_clips_normatively` | 313-323 | REAL | — | Ran ok (:709). Normative clip both rails. |
| test `horizontal_ramp_highest_frequency` | 325-350 | REAL | — | Ran ok (:711). Anti-symmetry + row equality — verifies the odd/even split's mirror logic. |
| test `quantization_scales_the_dc` | 352-362 | REAL | — | Ran ok (:715). Rounding path (4.25→4) exercised. |
| test `hostile_coefficients_cannot_overflow` | 364-383 | REAL | — | Ran ok (:712). ±32768×65535 all slots; completion + saturation asserted. In release-mode baseline a silent wrap would produce mid-range samples → assert fails. |
| test `short_output_slices_are_written_prefix_only` | 385-393 | REAL | — | Ran ok (:716). Covers len<64 only — the >64 asymmetry (F-001) is untested. |
| test `round_trips_against_a_reference_forward_dct` | 395-446 | REAL | — | Ran ok (:716). FDCT re-implemented inline with same rounding; bound ±8 documented and justified. |
| `TestRng` + `next_u64` | 450-461 | REAL | — | xorshift64*; test-only; no unwrap/panic paths. |
| test `factored_idct_is_bit_exact_vs_naive` | 463-557 | REAL | — | Ran ok (:710). THE test: structured patterns + 8192 random blocks × random tables, assert_eq!(exact, total). Would fail on ANY factoring divergence — mutation: swap A02↔A16 (line 86/89) → pe[0]/pe[1] terms wrong → immediate divergence. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| factored_idct_is_bit_exact_vs_naive | YES | Bit-equality against an independent naive implementation over randomized inputs; any constant error, butterfly sign error, or mirror index error diverges ≥1 sample. |
| hostile_coefficients_cannot_overflow | YES (release) | Saturated-only assertion; a wrapped intermediate yields non-0/255 samples. In debug a panic fails the run. |
| round_trips_against_a_reference_forward_dct | YES | Independent FDCT inline; ±8 bound would be exceeded by any basis error (basis errors scale with amplitude). |
| pure_dc/negative/dc_saturation/quantization (4) | YES | Exact expected constants re-derived by auditor; any scale/shift error moves them. |
| short_output_slices_are_written_prefix_only | PARTIAL | Covers the short-slice half of the prefix contract; the >64 half is unwritten (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bit-exact vs the naive form (pinned by factored_idct_is_bit_exact_vs_naive)" | idct.rs:34-36 | REAL | Baseline :710 ok; auditor re-derivation confirms exactness is algebraic, not accidental. |
| "committed reference digests in crates/codecs/tests" pin end-to-end output | idct.rs:36-37 | REAL | tests/fixtures_parent.rs exists w/ sha256 pins; baseline fixtures --check PASS (INDEX.md #12). |
| "i64 arithmetic cannot overflow for any input" | idct.rs:58 | REAL | Auditor re-computed: 8·32138·(8·32138·2²⁰) ≈ 6.9·10¹⁶ < 2⁶³≈9.2·10¹⁸; plus hostile test green. |
| "Integer-only, no floats anywhere" | idct.rs:21-22 | REAL | rg: the only f64 is inside the TEST random-density selector (:531-533), cfg(test) — production code clean. |

## 7. Dead / duplicated / unreachable code
None. `DCT_A`/`idct_block_naive` are cfg(test) by design (oracle), documented as such (:69-71, :244-246). Every pub(crate) item has an in-crate consumer (idct_simd.rs:141/218; decoder.rs via dispatch).

## 8. Dependencies & flags
No external deps, no unsafe, no feature gates — compiled unconditionally with the in-house mjpeg module (crates/codecs/src/video/mod.rs:35). The SIMD dispatch sibling (`idct_simd.rs`, A04's file) calls into this file as the default/fallback arm and reuses `shr32_round`/`idct8` — same-code bit-exactness by construction, not re-derivation (idct.rs:103-104, 121-122).

## 9. Open questions for the next agent
- idct_simd.rs (A04): does the SSE4.1 kernel's pass 1 preserve the exact clamp-then-sum order (clamp BEFORE the transform, as here :216)? A clamp-after would break bit-exactness only for values between the SIMD lane bound and MAX_COEFF.
- decoder.rs plane writer: does `Plane::write_block` crop partial edge blocks by the same prefix rule this file assumes (out ≤ 64)?
