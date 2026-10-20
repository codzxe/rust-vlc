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
//! # Overflow safety (fuzz guarantee)
//!
//! Dequantized coefficients are clamped to ±[`MAX_COEFF`] first.
//! Legitimate 8-bit baseline coefficients satisfy
//! `|q·Q| ≤ 2047 · 255 ≈ 5.2·10⁵` (category ≤ 11 for 8-bit samples,
//! quant values 1..255), far below the clamp; hostile bitstreams can
//! produce up to `±(2¹⁵−1) · 65535 ≈ 2.1·10⁹`, and cumulative DC
//! prediction grows that further. With the clamp, the worst inner
//! accumulation is bounded by
//! `8 · 32138 · MAX_COEFF · 8 · 32138 ≈ 6.9·10¹⁶ < 2⁶³`, so the i64
//! arithmetic below cannot overflow for any input (sample clipping
//! makes the clamp invisible on legitimate data — it is an
//! implementation safety bound, not a spec deviation).

/// Clamp on dequantized coefficients (safety bound; see module docs).
pub(crate) const MAX_COEFF: i64 = 1 << 20;

/// Forward-DCT basis in Q16 (identical to the 2.A3 encoder's `DCT_A`;
/// `A[u][x] = c(u)/2 · cos((2x+1) u π / 16)`, row-major `[u*8+x]`).
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

/// Divide by 2³² with round-half-away-from-zero.
fn shr32_round(value: i64) -> i64 {
    if value >= 0 {
        (value + (1i64 << 31)) >> 32
    } else {
        -((-value + (1i64 << 31)) >> 32)
    }
}

/// Dequantize + inverse-DCT one block into `out` samples.
///
/// `block` holds natural-order quantized coefficients (`F[v*8+u]`,
/// u horizontal); `qt` the natural-order quantization table; `out`
/// receives `w*h ≤ 64` row-major samples.
pub(crate) fn idct_block(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
    // Dequantize with the safety clamp (module docs: fuzz-bound).
    let mut d = [0i64; 64];
    for i in 0..64 {
        let raw = i64::from(block[i]) * i64::from(u32::from(qt[i]));
        d[i] = raw.clamp(-MAX_COEFF, MAX_COEFF);
    }

    // Pass 1 (columns of the coefficient matrix): mid[u][y] =
    // Σ_v A[v][y] · D[v][u].
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

    // Pass 2 (rows): f'[y][x] = Σ_u A[u][x] · mid[u][y], scaled back
    // down, level-shifted and clipped (F.2.2.4 — clipping is
    // normative).
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
        // The decoder uses this to write into padded planes; a short
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
}
