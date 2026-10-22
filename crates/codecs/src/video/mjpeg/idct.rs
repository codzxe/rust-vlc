//! Fixed-point inverse DCT (ISO/IEC 10918-1 §A.3.3 / F.2.2.1, cited
//! public specification — clean-room).
//!
//! The transform basis is the **same Q16 table the in-repo 2.A3
//! encoder uses** (`tools/xtask/src/cmd/fixtures.rs`, `DCT_A`):
//! `A[u][x] = c(u)/2 · cos((2x+1)·u·π/16)` rounded to Q16, with
//! `c(0) = 1/√2`. The encoder computes
//!
//! ```text
//! F[v][u] ≈ (Σ_y A[v][y] · Σ_x A[u][x] · (f − 128)) / 2³²
//! ```
//!
//! so this decoder computes the exact algebraic inverse,
//!
//! ```text
//! f′ = (Σ_u A[u][x] · Σ_v A[v][y] · D[v][u]) / 2³²
//! ```
//!
//! with round-half-away-from-zero (mirroring the encoder's rounding),
//! `+128` level shift, and the normative sample clip to `0..=255`
//! (§F.2.2.4). Integer-only, no floats anywhere (corpus-plan §2
//! discipline) — decode output is deterministic across platforms.
//!
//! # Factored form (ED-FIX-1, audit C1)
//!
//! The naive evaluation is two 8×8×8 matrix multiplies (1024 i64
//! MACs per block). The basis rows have exact symmetry structure —
//! `A[v][7−x] = +A[v][x]` for even `v`, `−A[v][x]` for odd `v` — so
//! each 8-point transform factors into an even-frequency part and an
//! odd-frequency part computed on half the outputs (`out[x] = pe[x] +
//! po[x]`, `out[7−x] = pe[x] − po[x]`), with the even part further
//! sharing the `A[0]/A[4]` butterflies. The factored form is a pure
//! algebraic **reordering of the same integer sums**: no rescaling,
//! no intermediate truncation, so it is **bit-exact** vs the naive
//! form (pinned by `factored_idct_is_bit_exact_vs_naive` over
//! randomized blocks, and end-to-end by the committed reference
//! digests in `crates/codecs/tests`). Dequantization is folded into
//! pass 1 (the per-column `w` inputs), and an all-AC-zero block —
//! the common case in real Motion-JPEG — takes a direct DC placement
//! fast path that skips both passes entirely.
//!
//! # Overflow safety (fuzz guarantee)
//!
//! Dequantized coefficients are clamped to ±[`MAX_COEFF`] first.
//! Legitimate 8-bit baseline coefficients satisfy
//! `|q·Q| ≤ 2047 · 255 ≈ 5.2·10⁵` (category ≤ 11 for 8-bit samples,
//! quant values 1..255), far below the clamp; hostile bitstreams can
//! produce up to `±(2¹⁵−1) · 65535 ≈ 2.1·10⁹`, and cumulative DC
//! prediction grows that further. With the clamp, the factored
//! intermediates are bounded by
//!
//! ```text
//! |pe|, |po| ≤ 4 · 32138 · 2²⁰ ≈ 1.35·10¹¹   (per 8-point output)
//! |mid|      ≤ 2.7·10¹¹                       (pass-1 output)
//! |pass-2 out| ≤ 8 · 32138 · 2.7·10¹¹ ≈ 7·10¹⁶ < 2⁶³
//! ```
//!
//! so the i64 arithmetic cannot overflow for any input (sample
//! clipping makes the clamp invisible on legitimate data — it is an
//! implementation safety bound, not a spec deviation). i32 is used
//! only where values are provably small (the final 8-bit samples;
//! the color path).

/// Clamp on dequantized coefficients (safety bound; see module docs).
pub(crate) const MAX_COEFF: i64 = 1 << 20;

/// Forward-DCT basis in Q16 (identical to the 2.A3 encoder's `DCT_A`;
/// `A[u][x] = c(u)/2 · cos((2x+1) u π / 16)`, row-major `[u*8+x]`).
/// Test-only since ED-FIX-1: the factored transform uses the split
/// constants below; this table remains the equivalence oracle's
/// ground truth (`idct_block_naive`, the round-trip property test).
#[cfg(test)]
const DCT_A: [i64; 64] = [
    23170, 23170, 23170, 23170, 23170, 23170, 23170, 23170, //
    32138, 27246, 18205, 6393, -6393, -18205, -27246, -32138, //
    30274, 12540, -12540, -30274, -30274, -12540, 12540, 30274, //
    27246, -6393, -32138, -18205, 18205, 32138, 6393, -27246, //
    23170, -23170, -23170, 23170, 23170, -23170, -23170, 23170, //
    18205, -32138, 6393, 27246, -27246, -6393, 32138, -18205, //
    12540, -30274, 30274, -12540, -12540, 30274, -30274, 12540, //
    6393, -18205, 27246, -32138, 32138, -27246, 18205, -6393,
];

/// Even-frequency row constants (Q16): `A[0][x] = A[4][x] = ±23170`,
/// and the `A[2]` / `A[6]` column pairs for `x ∈ 0..4`.
const A02: i64 = 30274; // A[2][0], -A[2][3]
const A12: i64 = 12540; // A[2][1], -A[2][2]
const A06: i64 = 12540; // A[6][0], -A[6][3]
const A16: i64 = 30274; // A[6][1], -A[6][2]
const A0: i64 = 23170; // A[0][*] = A[4][*] magnitude

/// Odd-frequency row constants (Q16) for `x ∈ 0..4` (the remaining
/// four outputs come out by antisymmetry).
const O1: [i64; 4] = [32138, 27246, 18205, 6393]; // A[1][0..4]
const O3: [i64; 4] = [27246, -6393, -32138, -18205]; // A[3][0..4]
const O5: [i64; 4] = [18205, -32138, 6393, 27246]; // A[5][0..4]
const O7: [i64; 4] = [6393, -18205, 27246, -32138]; // A[7][0..4]

/// `A[0][·]²` in Q32 — the DC-only fast-path scale (`23170²`).
const DC_SCALE: i64 = 23170 * 23170;

/// Divide by 2³² with round-half-away-from-zero.
fn shr32_round(value: i64) -> i64 {
    if value >= 0 {
        (value + (1i64 << 31)) >> 32
    } else {
        -((-value + (1i64 << 31)) >> 32)
    }
}

/// The factored exact 8-point transform: `out[x] = Σ_v A[v][x] · w[v]`.
///
/// Pure reordering of the naive dot products (module docs): the
/// even-frequency inputs (`w[0,2,4,6]`) contribute a symmetric half
/// (`pe[7−x] = pe[x]`), the odd-frequency inputs an antisymmetric
/// half (`po[7−x] = −po[x]`), so only `x ∈ 0..4` is computed and the
/// rest mirrors. Worst case 22 i64 multiplies vs the naive 64;
/// all-zero frequency groups skip their share.
#[inline]
fn idct8(w: &[i64; 8]) -> [i64; 8] {
    let mut pe = [0i64; 4];
    let mut po = [0i64; 4];

    // Even frequencies: pe[x] = Σ_{v even} A[v][x]·w[v], computed on
    // the (w0 ± w4) butterflies (A[0] and A[4] share the magnitude
    // 23170 with opposite signs past x = 0).
    if (w[0] | w[4]) != 0 {
        let s = A0 * (w[0] + w[4]); // pe[0], pe[3]
        let d = A0 * (w[0] - w[4]); // pe[1], pe[2]
        pe[0] += s;
        pe[3] += s;
        pe[1] += d;
        pe[2] += d;
    }
    if w[2] != 0 {
        let hi = A02 * w[2]; // pe[0], -pe[3]
        let lo = A12 * w[2]; // pe[1], -pe[2]
        pe[0] += hi;
        pe[3] -= hi;
        pe[1] += lo;
        pe[2] -= lo;
    }
    if w[6] != 0 {
        let hi = A16 * w[6]; // -pe[1], pe[2]
        let lo = A06 * w[6]; // pe[0], -pe[3]
        pe[0] += lo;
        pe[3] -= lo;
        pe[1] -= hi;
        pe[2] += hi;
    }

    // Odd frequencies: po[x] = Σ_{v odd} A[v][x]·w[v] (dense 4×4 —
    // the exact-integer sharing below the symmetry split requires
    // non-integer half-coefficients, so this stays a direct product).
    if w[1] != 0 {
        for (slot, &a) in po.iter_mut().zip(O1.iter()) {
            *slot += a * w[1];
        }
    }
    if w[3] != 0 {
        for (slot, &a) in po.iter_mut().zip(O3.iter()) {
            *slot += a * w[3];
        }
    }
    if w[5] != 0 {
        for (slot, &a) in po.iter_mut().zip(O5.iter()) {
            *slot += a * w[5];
        }
    }
    if w[7] != 0 {
        for (slot, &a) in po.iter_mut().zip(O7.iter()) {
            *slot += a * w[7];
        }
    }

    [
        pe[0] + po[0],
        pe[1] + po[1],
        pe[2] + po[2],
        pe[3] + po[3],
        pe[3] - po[3],
        pe[2] - po[2],
        pe[1] - po[1],
        pe[0] - po[0],
    ]
}

/// Dequantize + inverse-DCT one block into `out` samples.
///
/// `block` holds natural-order quantized coefficients (`F[v*8+u]`,
/// u horizontal); `qt` the natural-order quantization table; `out`
/// receives `w*h ≤ 64` row-major samples.
pub(crate) fn idct_block(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
    // Zero-AC fast path: every real-world flat block (the majority
    // at typical Motion-JPEG quantization) reduces to a constant —
    // only the u = v = 0 basis term survives, scaled by A[0]².
    if block[1..].iter().all(|&q| q == 0) {
        let w0 = (i64::from(block[0]) * i64::from(u32::from(qt[0]))).clamp(-MAX_COEFF, MAX_COEFF);
        let sample = (shr32_round(w0 * DC_SCALE) + 128).clamp(0, 255) as u8;
        out.fill(sample);
        return;
    }

    // Pass 1 (columns of the coefficient matrix) with dequantization
    // folded in: mid[u][y] = Σ_v A[v][y] · w_col(v), where
    // w_col(v) = clamp(q · Q) for the column's v-th coefficient.
    let mut mid = [0i64; 64];
    for u in 0..8 {
        let mut w = [0i64; 8];
        for v in 0..8 {
            let raw = i64::from(block[v * 8 + u]) * i64::from(u32::from(qt[v * 8 + u]));
            w[v] = raw.clamp(-MAX_COEFF, MAX_COEFF);
        }
        let col = idct8(&w);
        mid[u * 8..u * 8 + 8].copy_from_slice(&col);
    }

    // Pass 2 (rows): f'[y][x] = Σ_u A[u][x] · mid[u][y], scaled back
    // down, level-shifted and clipped (F.2.2.4 — clipping is
    // normative). `mid` is transposed on the fly (row y of the output
    // reads strided mid[u*8 + y]).
    let mut samples = [0u8; 64];
    for y in 0..8 {
        let mut w = [0i64; 8];
        for u in 0..8 {
            w[u] = mid[u * 8 + y];
        }
        let row = idct8(&w);
        for x in 0..8 {
            samples[y * 8 + x] = (shr32_round(row[x]) + 128).clamp(0, 255) as u8;
        }
    }

    // Prefix write (the decoder writes into compact planes; a short
    // slice must not panic and fills what fits).
    let n = out.len().min(64);
    out[..n].copy_from_slice(&samples[..n]);
}

/// The pre-ED-FIX-1 naive reference: two full 8×8×8 matrix
/// multiplies. Test-only — the equivalence oracle for the factored
/// form (Lead decision ED-c).
#[cfg(test)]
fn idct_block_naive(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
    let mut d = [0i64; 64];
    for i in 0..64 {
        let raw = i64::from(block[i]) * i64::from(u32::from(qt[i]));
        d[i] = raw.clamp(-MAX_COEFF, MAX_COEFF);
    }
    let mut mid = [0i64; 64];
    for u in 0..8 {
        for y in 0..8 {
            let mut acc = 0i64;
            for v in 0..8 {
                acc += DCT_A[v * 8 + y] * d[v * 8 + u];
            }
            mid[u * 8 + y] = acc;
        }
    }
    for y in 0..8 {
        for x in 0..8 {
            let mut acc = 0i64;
            for u in 0..8 {
                acc += DCT_A[u * 8 + x] * mid[u * 8 + y];
            }
            let sample = shr32_round(acc) + 128;
            if let Some(slot) = out.get_mut(y * 8 + x) {
                *slot = sample.clamp(0, 255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNITY_QT: [u16; 64] = [1; 64];

    #[test]
    fn zero_block_is_flat_mid_gray() {
        let block = [0i16; 64];
        let mut out = [0u8; 64];
        idct_block(&block, &UNITY_QT, &mut out);
        assert!(out.iter().all(|&s| s == 128), "{out:?}");
    }

    #[test]
    fn pure_dc_block_is_constant() {
        // A DC-only block is the constant F(0,0)/8: 40/8 = 5 above
        // mid-gray → 133 everywhere (§A.3.3: DC scales by 1/8 of the
        // block sum).
        let mut block = [0i16; 64];
        block[0] = 40;
        let mut out = [0u8; 64];
        idct_block(&block, &UNITY_QT, &mut out);
        assert!(out.iter().all(|&s| s == 133), "{out:?}");
    }

    #[test]
    fn negative_dc_shifts_down() {
        let mut block = [0i16; 64];
        block[0] = -40;
        let mut out = [0u8; 64];
        idct_block(&block, &UNITY_QT, &mut out);
        assert!(out.iter().all(|&s| s == 123), "{out:?}");
    }

    #[test]
    fn dc_saturation_clips_normatively() {
        let mut block = [0i16; 64];
        block[0] = 4096;
        let mut out = [0u8; 64];
        idct_block(&block, &UNITY_QT, &mut out);
        assert!(out.iter().all(|&s| s == 255), "{out:?}");
        block[0] = -4096;
        idct_block(&block, &UNITY_QT, &mut out);
        assert!(out.iter().all(|&s| s == 0), "{out:?}");
    }

    #[test]
    fn horizontal_ramp_highest_frequency() {
        // AC(0,7): the (1,-1,-1,...)-ish checkerboard at the highest
        // horizontal frequency; amplitude 30 with Q=1.
        let mut block = [0i16; 64];
        block[7] = 30; // F[v=0][u=7]
        let mut out = [0u8; 64];
        idct_block(&block, &UNITY_QT, &mut out);
        // Row values must alternate symmetric around 128: x and 7-x
        // have opposite sign contributions.
        for y in 0..8 {
            for x in 0..8 {
                let mirror = out[y * 8 + (7 - x)];
                let here = out[y * 8 + x];
                assert_eq!(
                    here as i16 + mirror as i16,
                    256,
                    "anti-symmetric pair at y={y} x={x}: {here} + {mirror}"
                );
            }
        }
        // And every row is identical (vertical frequency 0).
        for y in 1..8 {
            assert_eq!(&out[y * 8..y * 8 + 8], &out[0..8], "row {y} equals row 0");
        }
    }

    #[test]
    fn quantization_scales_the_dc() {
        // q=17, Q=2 → D=34 → constant 34/8 = 4.25 → 4 above 128.
        let mut block = [0i16; 64];
        block[0] = 17;
        let mut qt = [1u16; 64];
        qt[0] = 2;
        let mut out = [0u8; 64];
        idct_block(&block, &qt, &mut out);
        assert!(out.iter().all(|&s| s == 132), "{out:?}");
    }

    #[test]
    fn hostile_coefficients_cannot_overflow() {
        // Max-magnitude coefficients in every slot: must complete
        // without panic (the u8 output is in range by construction —
        // the assertion under test is completion itself).
        let block = [-32768i16; 64];
        let qt = [65535u16; 64];
        let mut out = [0u8; 64];
        idct_block(&block, &qt, &mut out);
        assert!(
            out.iter().all(|&s| s == 0 || s == 255),
            "saturated: {out:?}"
        );
        let block = [32767i16; 64];
        idct_block(&block, &qt, &mut out);
        assert!(
            out.iter().all(|&s| s == 0 || s == 255),
            "saturated: {out:?}"
        );
    }

    #[test]
    fn short_output_slices_are_written_prefix_only() {
        // The decoder uses this to write into compact planes; a short
        // slice must not panic and must fill what fits.
        let block = [0i16; 64];
        let mut out = [17u8; 24];
        idct_block(&block, &UNITY_QT, &mut out);
        assert!(out.iter().all(|&s| s == 128), "{out:?}");
    }

    #[test]
    fn round_trips_against_a_reference_forward_dct() {
        // Property: IDCT is the algebraic inverse of the encoder's
        // FDCT for coefficient magnitudes well inside the clamp: pick
        // samples, forward-transform with the encoder's exact integer
        // math, dequantize (Q=1 keeps it exact-ish) and invert.
        let mut samples = [0u8; 64];
        for (i, slot) in samples.iter_mut().enumerate() {
            *slot = ((i * 37 + 11) % 256) as u8;
        }
        // Encoder FDCT (fixtures.rs math, Q=1 → quantized == rounded F).
        let mut mid = [0i64; 64];
        for y in 0..8 {
            for u in 0..8 {
                let mut acc = 0i64;
                for x in 0..8 {
                    acc += DCT_A[u * 8 + x] * (i64::from(samples[y * 8 + x]) - 128);
                }
                mid[u * 8 + y] = acc;
            }
        }
        let mut coeffs = [0i16; 64];
        for u in 0..8 {
            for v in 0..8 {
                let mut acc = 0i64;
                for y in 0..8 {
                    acc += DCT_A[v * 8 + y] * mid[u * 8 + y];
                }
                // Q=1: quantized value = round(F).
                let rounded = if acc >= 0 {
                    (acc + (1i64 << 31)) >> 32
                } else {
                    -((-acc + (1i64 << 31)) >> 32)
                };
                coeffs[v * 8 + u] = i16::try_from(rounded.clamp(-2048, 2047)).unwrap_or(0);
            }
        }
        let mut out = [0u8; 64];
        idct_block(&coeffs, &UNITY_QT, &mut out);
        // The FDCT rounding floor at Q=1 is ±0.5 per coefficient; the
        // inverse basis products reach ~0.24, so adversarially aligned
        // rounding could deviate up to ~8 on a pixel. Assert that
        // documented bound (the observed max for this pattern is far
        // smaller — the point is the inverse property, not tightness).
        for (i, (&got, &want)) in out.iter().zip(samples.iter()).enumerate() {
            let delta = i16::from(got) - i16::from(want);
            assert!(
                delta.abs() <= 8,
                "sample {i}: {delta} (out {got}, want {want})"
            );
        }
    }

    /// Deterministic xorshift64* for the equivalence sweep (test-only;
    // no external RNG, no HashMap — corpus-plan §2 discipline).
    struct TestRng(u64);

    impl TestRng {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }
    }

    #[test]
    fn factored_idct_is_bit_exact_vs_naive() {
        // ED-FIX-1 exactness contract (Lead decision ED-c): the
        // factored transform is a pure reordering of the naive sums,
        // so every sample must match bit-exactly. Sweep structured
        // patterns (zero, DC-only, single-AC spikes at every zigzag
        // position, saturation) plus 8192 pseudo-random blocks under
        // 64 pseudo-random quant tables, including hostile magnitude
        // mixes that exercise the ±MAX_COEFF clamp and the i64
        // overflow headroom. The committed-corpus half of the evidence
        // is end-to-end: tests/fixtures_parent.rs pins every frame's
        // sha256 against digests produced by the pre-factoring
        // decoder (bit-exact there means bit-exact here, transitively
        // over every block of the corpus).
        let mut rng = TestRng(0x5eed_1234_abcd_ef01);
        let mut exact = 0usize;
        let mut total = 0usize;
        let mut diverged = Vec::new();

        let mut check = |block: &[i16; 64], qt: &[u16; 64], tag: &str| {
            let mut fast = [0u8; 64];
            let mut naive = [0u8; 64];
            idct_block(block, qt, &mut fast);
            idct_block_naive(block, qt, &mut naive);
            total += 64;
            if fast == naive {
                exact += 64;
            } else {
                for (i, (&a, &b)) in fast.iter().zip(naive.iter()).enumerate() {
                    if a != b {
                        diverged.push(format!("{tag} sample {i}: {a} vs {b}"));
                    }
                }
            }
        };

        // Structured patterns.
        check(&[0i16; 64], &UNITY_QT, "zero");
        for &dc in &[0i16, 1, -1, 40, -40, 2047, -2048, 32767, -32768] {
            let mut block = [0i16; 64];
            block[0] = dc;
            check(&block, &UNITY_QT, "dc-only");
            let mut qt = [16u16; 64];
            qt[0] = 255;
            check(&block, &qt, "dc-only/q255");
        }
        for pos in 0..64 {
            for &amp in &[1i16, -1, 300, -300, 32767, -32768] {
                let mut block = [0i16; 64];
                block[pos] = amp;
                let mut qt = [1u16; 64];
                qt[pos] = 99;
                check(&block, &qt, "single-ac");
            }
        }
        check(&[-32768i16; 64], &[65535u16; 64], "hostile-max");
        check(&[32767i16; 64], &[65535u16; 64], "hostile-max-pos");
        // Every-coefficient-nonzero gradient (full-rank stress).
        let mut block = [0i16; 64];
        for (i, slot) in block.iter_mut().enumerate() {
            *slot = ((i as i32 * 73) % 4096 - 2048) as i16;
        }
        check(&block, &[7u16; 64], "gradient");

        // Randomized sweep (sparsity mix mirrors real entropy: many
        // zero ACs, occasional dense hostile blocks).
        for round in 0..8192 {
            let mut block = [0i16; 64];
            let density = [0.0f64, 0.02, 0.1, 0.4, 1.0][round % 5];
            for slot in block.iter_mut() {
                if (rng.next_u64() as f64 / u64::MAX as f64) < density {
                    *slot = (rng.next_u64() % 65536) as i16;
                }
            }
            let mut qt = [0u16; 64];
            for slot in qt.iter_mut() {
                *slot = (1 + rng.next_u64() % 255) as u16;
            }
            check(&block, &qt, "random");
        }

        assert!(
            diverged.is_empty(),
            "factored IDCT diverged from the naive reference in {} sample(s):\n{}",
            diverged.len(),
            diverged.join("\n")
        );
        // Bit-exact is the required outcome (pure reordering); the
        // count is printed as evidence, not just asserted.
        println!(
            "factored-vs-naive equivalence: {exact}/{total} samples bit-exact ({} divergences)",
            diverged.len()
        );
        assert_eq!(exact, total);
    }
}
