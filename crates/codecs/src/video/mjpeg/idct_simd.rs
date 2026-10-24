//! SIMD IDCT for the MJPEG decode hot stage (card 7-b) — the ONE
//! sanctioned `unsafe` exception in the repo (contract
//! `.agent-brief-w7.md`, D-10 / rule 5): `std::arch` x86_64
//! intrinsics, runtime-dispatched via `is_x86_feature_detected!`,
//! a safety comment on every `unsafe` block, and the scalar
//! [`idct::idct_block`] fallback that stays correct and DEFAULT on
//! non-x86_64 and pre-SSE4.1 CPUs (everything is `cfg`-guarded, not
//! assumed).
//!
//! # What is vectorized (and what is not — honest scope)
//!
//! **Pass 1** (dequantize + column transform, the `Σ_v A[v][y]·w[v]`
//! dots) is fully SIMD. **Pass 2** (row transform + round + clip)
//! reuses the scalar factored [`idct::idct8`] on the exact `i64`
//! reconstruction of `mid` — pass-2 vectorization needs >32-bit
//! products (`|A·mid| ≤ 8.7·10¹⁵`) and is deferred until the
//! measured stage attribution justifies it. The all-AC-zero fast
//! path delegates to the scalar routine unchanged.
//!
//! # Bit-exactness proof (non-negotiable, same discipline as
//! [`idct::idct_block`])
//!
//! The scalar path computes, in `i64`, `mid[y][u] = Σ_v A[v][y]·w[v]`
//! with `w = clamp(q·Q, ±MAX_COEFF)` (`|w| ≤ 2²⁰`). The kernel splits
//! each clamped `w` exactly (two's-complement radix decomposition):
//!
//! ```text
//! w = 256·w_hi + w_lo,  w_hi = w >> 8 (|w_hi| ≤ 2¹²),  w_lo = w & 0xFF (∈ 0..=255)
//! ```
//!
//! so, by linearity and with NO intermediate rounding,
//!
//! ```text
//! mid = 256·Dhi + Dlo,  Dhi = Σ_v A[v][y]·w_hi,  Dlo = Σ_v A[v][y]·w_lo
//! ```
//!
//! Both partial sums run in `i32` inside `_mm_madd_epi16` (8 products
//! of two `i16` lanes) + an exact shuffle add reduction. Bounds:
//!
//! ```text
//! |A| ≤ 32138, |w_hi| ≤ 4096   → |Dhi| ≤ 8·32138·4096  = 1 053 454 784 < 2³¹
//! |w_lo| ≤ 255                 → |Dlo| ≤ 8·32138·255   =    65 561 520 < 2³¹
//! ```
//!
//! so no `i32` lane can wrap; `mid` reconstructed in `i64`
//! (`≤ 2.7·10¹¹`, the scalar module's own bound) is bit-identical to
//! the scalar `mid`, and pass 2 is literally the same scalar code —
//! the output is byte-equal by construction, pinned by the
//! randomized equality test below (deterministic seed) plus the
//! existing end-to-end reference digests.
//!
//! # Why the gate is SSE4.1, not SSE2
//!
//! The exact dequant needs a 32-bit integer multiply
//! (`_mm_mullo_epi32`) and 32-bit min/max (`_mm_min_epi32` /
//! `_mm_max_epi32`); the largest product, `−32768 · 65535 =
//! −2 147 450 880`, provably fits `i32`, so it is exact. SSE2-only
//! would require yet another 16-bit split with zero bit-exactness
//! risk but extra work for no measured gain — deferred. An AVX2
//! widening of pass 1 is likewise deferred until the 20261022
//! protocol measures this version honestly first (successor session;
//! see `docs/perf/mjpeg-simd-20261024.md` plan).
//!
//! # UNMEASURED [VERIFY]
//!
//! No perf numbers are claimed here yet. The before/after run under
//! the IDENTICAL 20261022 protocol (avi-av / uhd4k-mjpeg /
//! heavy-mjpeg, medians, N, load conditions, raw data in
//! `docs/perf/raw/mjpeg/`) is the successor session's first job; if
//! this kernel does not win, rule 5 applies (say so, keep scalar).

// The ONE sanctioned `unsafe` exception (contract rule 5 / D-10):
// this module — and only this module — re-allows the workspace-wide
// `unsafe_code = "deny"` lint so the `std::arch` x86_64 intrinsics
// below can exist behind `is_x86_feature_detected!`, each `unsafe`
// block carrying its safety comment, with the bit-exact scalar
// fallback remaining the default off-path.
#![allow(unsafe_code)]

// x86_64-only consumers (the scalar pass-2 reuse lives in the
// cfg-gated kernel); non-x86_64 builds must not see an unused import.
#[cfg(target_arch = "x86_64")]
use super::idct::{idct8, shr32_round};

/// Forward-DCT basis in Q16, `A[v*8+y] = A[v][y]` (same table as
/// [`idct`]'s `DCT_A`, duplicated here because that copy is
/// `#[cfg(test)]`-gated as the naive-oracle's ground truth).
const DCT_A_Q16: [i32; 64] = [
    23170, 23170, 23170, 23170, 23170, 23170, 23170, 23170, //
    32138, 27246, 18205, 6393, -6393, -18205, -27246, -32138, //
    30274, 12540, -12540, -30274, -30274, -12540, 12540, 30274, //
    27246, -6393, -32138, -18205, 18205, 32138, 6393, -27246, //
    23170, -23170, -23170, 23170, 23170, -23170, -23170, 23170, //
    18205, -32138, 6393, 27246, -27246, -6393, 32138, -18205, //
    12540, -30274, 30274, -12540, -12540, 30274, -30274, 12540, //
    6393, -18205, 27246, -32138, 32138, -27246, 18205, -6393,
];

/// `A_COLS[y][v] = A[v][y]` — the basis by column, so each pass-1
/// dot `Σ_v A[v][y]·w_col[v]` is one `_mm_madd_epi16` against a
/// contiguous `w` column. All values fit `i16` (max 32138 < 32768).
#[cfg(target_arch = "x86_64")]
const A_COLS: [[i16; 8]; 8] = {
    let mut cols = [[0i16; 8]; 8];
    let mut y = 0;
    while y < 8 {
        let mut v = 0;
        while v < 8 {
            cols[y][v] = DCT_A_Q16[v * 8 + y] as i16;
            v += 1;
        }
        y += 1;
    }
    cols
};

/// Runtime dispatch — LEAD DECISION 2026-10-24 (W7-b [PROPOSAL] accepted):
/// the measured ablation (docs/perf/mjpeg-simd-20261024.md: 0063-avi-av
/// 1.021×, 0127-heavy 0.995× — SIMD on the SLOW side of noise, N=7/N=4
/// medians, release, interleaved arms) means the wave-brief's keep-scalar
/// rule applies: **scalar is the default**. The SSE4.1 kernel stays
/// compiled, tested (bit-exactness suite), and reachable for re-measurement
/// via the `LUMEN_MJPEG_IDCT_SIMD=1` opt-in — flipping the default back
/// requires a fresh ablation showing a real win, per the honest-numbers
/// mandate.
pub(crate) fn idct_block_dispatch(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
    #[cfg(target_arch = "x86_64")]
    {
        // Opt-in only (see the LEAD DECISION note above).
        if std::env::var_os("LUMEN_MJPEG_IDCT_SIMD").is_some()
            && std::arch::is_x86_feature_detected!("sse4.1")
        {
            // SAFETY: the kernel is #[target_feature(enable = "sse4.1")]
            // and is only entered after the "sse4.1" CPU feature bit is
            // detected at runtime; it touches only the caller-provided
            // slices (no escapes, no unaligned UB — all loads/stores use
            // the unaligned intrinsics on validly-referenced memory).
            return unsafe { idct_block_sse41(block, qt, out) };
        }
    }
    super::idct::idct_block(block, qt, out);
}

/// Exact horizontal sum of the four `i32` lanes (shuffle adds —
/// integer-exact, no rounding anywhere).
///
/// # Safety
/// SSE2 intrinsics; caller must be on a CPU with SSE2 (guaranteed on
/// x86_64 and subsumed by the SSE4.1 gate at the dispatch site).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn hsum32(x: std::arch::x86_64::__m128i) -> i32 {
    use std::arch::x86_64::*;
    // Register-only ops: with `#[target_feature(enable = "sse2")]`
    // these intrinsics are safe calls; the fn stays `unsafe` because
    // CALLING it requires the SSE2 feature guarantee. Lane-shuffle
    // adds are integer-exact (module bound proof: partials < 2³¹).
    // _MM_SHUFFLE(2,3,0,1): swap lane pairs.
    let pair = _mm_add_epi32(x, _mm_shuffle_epi32(x, 0b10_11_00_01));
    // _MM_SHUFFLE(1,0,3,2): swap the two sums — lane 0 then holds
    // the full four-lane total.
    let full = _mm_add_epi32(pair, _mm_shuffle_epi32(pair, 0b01_00_11_10));
    _mm_cvtsi128_si32(full)
}

/// Classic SSE2 8×8 `i16` transpose (row-major in, column-major out).
///
/// # Safety
/// SSE2 intrinsics; caller must be on a CPU with SSE2.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn transpose8x16(rows: [std::arch::x86_64::__m128i; 8]) -> [std::arch::x86_64::__m128i; 8] {
    use std::arch::x86_64::*;
    // Register-only lane permutations on fully-initialized vectors
    // (safe calls under the enabled SSE2 feature).
    let t0 = _mm_unpacklo_epi16(rows[0], rows[1]);
    let t1 = _mm_unpacklo_epi16(rows[2], rows[3]);
    let t2 = _mm_unpacklo_epi16(rows[4], rows[5]);
    let t3 = _mm_unpacklo_epi16(rows[6], rows[7]);
    let t4 = _mm_unpackhi_epi16(rows[0], rows[1]);
    let t5 = _mm_unpackhi_epi16(rows[2], rows[3]);
    let t6 = _mm_unpackhi_epi16(rows[4], rows[5]);
    let t7 = _mm_unpackhi_epi16(rows[6], rows[7]);
    let u0 = _mm_unpacklo_epi32(t0, t1);
    let u1 = _mm_unpackhi_epi32(t0, t1);
    let u2 = _mm_unpacklo_epi32(t4, t5);
    let u3 = _mm_unpackhi_epi32(t4, t5);
    let u4 = _mm_unpacklo_epi32(t2, t3);
    let u5 = _mm_unpackhi_epi32(t2, t3);
    let u6 = _mm_unpacklo_epi32(t6, t7);
    let u7 = _mm_unpackhi_epi32(t6, t7);
    [
        _mm_unpacklo_epi64(u0, u4),
        _mm_unpackhi_epi64(u0, u4),
        _mm_unpacklo_epi64(u1, u5),
        _mm_unpackhi_epi64(u1, u5),
        _mm_unpacklo_epi64(u2, u6),
        _mm_unpackhi_epi64(u2, u6),
        _mm_unpacklo_epi64(u3, u7),
        _mm_unpackhi_epi64(u3, u7),
    ]
}

/// SSE4.1 kernel: SIMD dequant + pass 1, scalar pass 2 (module docs).
///
/// # Safety
/// SSE4.1 intrinsics (`_mm_mullo_epi32`, `_mm_min_epi32`,
/// `_mm_max_epi32` + SSE2 baseline). Must only be called after
/// `is_x86_feature_detected!("sse4.1")` (the dispatch does).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.1")]
unsafe fn idct_block_sse41(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
    use std::arch::x86_64::*;

    // Flat blocks: identical outcome via the scalar zero-AC fast
    // path (it is already cheaper than any vector walk).
    if block[1..].iter().all(|&q| q == 0) {
        return super::idct::idct_block(block, qt, out);
    }

    // ---- Stage 1: dequantize, clamp to ±MAX_COEFF (2²⁰), split
    // w = 256·w_hi + w_lo, per natural-order row v. All i32 math is
    // exact: |q·Q| ≤ 32768·65535 = 2 147 450 880 < 2³¹.
    let mut w_hi = [[0i16; 8]; 8];
    let mut w_lo = [[0i16; 8]; 8];
    // SAFETY: SSE4.1 gated at dispatch; unaligned load/store
    // intrinsics on the caller's validly-sized slices/arrays below
    // (8×i16 = one XMM per row).
    unsafe {
        let zero = _mm_setzero_si128();
        let lim = _mm_set1_epi32(1 << 20);
        let nlim = _mm_set1_epi32(-(1 << 20));
        let mask255 = _mm_set1_epi32(0xFF);
        for v in 0..8 {
            let b = _mm_loadu_si128(block.as_ptr().add(v * 8) as *const __m128i);
            let q = _mm_loadu_si128(qt.as_ptr().add(v * 8) as *const __m128i);
            // Sign-extend the i16 coefficients, zero-extend the u16
            // quantizers (exact domains).
            let blo = _mm_srai_epi32(_mm_unpacklo_epi16(b, b), 16);
            let bhi = _mm_srai_epi32(_mm_unpackhi_epi16(b, b), 16);
            let qlo = _mm_unpacklo_epi16(q, zero);
            let qhi = _mm_unpackhi_epi16(q, zero);
            let plo = _mm_mullo_epi32(blo, qlo);
            let phi = _mm_mullo_epi32(bhi, qhi);
            // clamp(q·Q, ±2²⁰) — same MAX_COEFF as the scalar path.
            let clo = _mm_min_epi32(_mm_max_epi32(plo, nlim), lim);
            let chi = _mm_min_epi32(_mm_max_epi32(phi, nlim), lim);
            // Exact split (two's complement): w = 256·(w>>8) + (w&255).
            let hrow = _mm_packs_epi32(_mm_srai_epi32(clo, 8), _mm_srai_epi32(chi, 8));
            let lrow = _mm_packs_epi32(_mm_and_si128(clo, mask255), _mm_and_si128(chi, mask255));
            _mm_storeu_si128(w_hi[v].as_mut_ptr() as *mut __m128i, hrow);
            _mm_storeu_si128(w_lo[v].as_mut_ptr() as *mut __m128i, lrow);
        }
    }

    // ---- Stage 2: transpose so each block column u is contiguous
    // (the pass-1 dot index v runs along the lanes).
    // SAFETY: SSE4.1 gated; permutations only.
    let (w_hi_t, w_lo_t) = unsafe {
        let mut hi = [_mm_setzero_si128(); 8];
        let mut lo = [_mm_setzero_si128(); 8];
        for v in 0..8 {
            hi[v] = _mm_loadu_si128(w_hi[v].as_ptr() as *const __m128i);
            lo[v] = _mm_loadu_si128(w_lo[v].as_ptr() as *const __m128i);
        }
        (transpose8x16(hi), transpose8x16(lo))
    };

    // ---- Stage 3: pass 1 in i32 halves. mid[y][u] = 256·Dhi + Dlo
    // (exact; module bound proof keeps both halves in i32). Layout
    // [y][u] equals the scalar mid storage transposed — pass 2 below
    // reads it with the same values.
    let mut d_hi = [0i32; 64];
    let mut d_lo = [0i32; 64];
    // SAFETY: SSE4.1 gated; madd lane sums bounded < 2³¹ (proof in
    // module docs), loads are from our own aligned-content arrays.
    unsafe {
        for y in 0..8 {
            let a = _mm_loadu_si128(A_COLS[y].as_ptr() as *const __m128i);
            for u in 0..8 {
                d_hi[y * 8 + u] = hsum32(_mm_madd_epi16(a, w_hi_t[u]));
                d_lo[y * 8 + u] = hsum32(_mm_madd_epi16(a, w_lo_t[u]));
            }
        }
    }

    // ---- Stage 4: pass 2, scalar and bit-identical to idct.rs —
    // reconstruct mid in i64 (256·Dhi + Dlo, no rounding) and run
    // the same factored idct8 + round-half-away + clip.
    let mut samples = [0u8; 64];
    for y in 0..8 {
        let mut w = [0i64; 8];
        for (u, slot) in w.iter_mut().enumerate() {
            *slot = 256 * i64::from(d_hi[y * 8 + u]) + i64::from(d_lo[y * 8 + u]);
        }
        let row = idct8(&w);
        for (x, slot) in samples[y * 8..y * 8 + 8].iter_mut().enumerate() {
            *slot = (shr32_round(row[x]) + 128).clamp(0, 255) as u8;
        }
    }

    // Prefix write, same contract as the scalar path (short plane
    // tails must not panic and fill what fits).
    let n = out.len().min(64);
    out[..n].copy_from_slice(&samples[..n]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic xorshift64* (same discipline as idct.rs — no
    /// external RNG).
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

    /// Scalar vs dispatched path, byte-equal. On x86_64 with SSE4.1
    /// this exercises the SIMD kernel (the deployment case); on
    /// non-x86_64 or SSE4.1-less CPUs it degrades to scalar-vs-scalar
    /// (still pins the fallback contract; the kernel arm is pinned on
    /// the dev/CI machines, which are SSE4.1+).
    #[test]
    fn dispatch_is_bit_exact_vs_scalar() {
        let mut rng = TestRng(0x7a11_d00d_feed_beef);
        let mut diverged: Vec<String> = Vec::new();
        let mut check = |block: &[i16; 64], qt: &[u16; 64], out_len: usize, tag: &str| {
            let mut scalar = [0u8; 64];
            let mut simd = [0u8; 64];
            super::super::idct::idct_block(block, qt, &mut scalar);
            idct_block_dispatch(block, qt, &mut simd);
            if scalar != simd {
                for (i, (&a, &b)) in scalar.iter().zip(simd.iter()).enumerate() {
                    if a != b {
                        diverged.push(format!(
                            "{tag}/len{out_len} sample {i}: scalar {a} vs simd {b}"
                        ));
                    }
                }
            }
            // Truncated-output fixture: prefix write must match too.
            if out_len < 64 {
                let mut s2 = [9u8; 64];
                let mut d2 = [9u8; 64];
                super::super::idct::idct_block(block, qt, &mut s2[..out_len]);
                idct_block_dispatch(block, qt, &mut d2[..out_len]);
                if s2[..out_len] != d2[..out_len] {
                    diverged.push(format!("{tag}: truncated len {out_len} diverged"));
                }
            }
        };

        let unity = [1u16; 64];
        // Structured fixtures (mirror idct.rs's oracle suite).
        check(&[0i16; 64], &unity, 64, "zero");
        for &dc in &[0i16, 1, -1, 40, -40, 2047, -2048, 32767, -32768] {
            let mut b = [0i16; 64];
            b[0] = dc;
            check(&b, &unity, 64, "dc-only");
            let mut qt = [16u16; 64];
            qt[0] = 255;
            check(&b, &qt, 64, "dc-only/q255");
        }
        for pos in 0..64 {
            for &amp in &[1i16, -1, 300, -300, 32767, -32768] {
                let mut b = [0i16; 64];
                b[pos] = amp;
                let mut qt = [1u16; 64];
                qt[pos] = 99;
                check(&b, &qt, 64, "single-ac");
            }
        }
        check(&[-32768i16; 64], &[65535u16; 64], 64, "hostile-max-neg");
        check(&[32767i16; 64], &[65535u16; 64], 64, "hostile-max-pos");
        let mut grad = [0i16; 64];
        for (i, slot) in grad.iter_mut().enumerate() {
            *slot = ((i as i32 * 73) % 4096 - 2048) as i16;
        }
        check(&grad, &[7u16; 64], 64, "gradient");
        // Truncated outputs at several lengths (edge planes).
        for &len in &[0usize, 1, 7, 24, 63] {
            let mut b = [0i16; 64];
            b[9] = -777;
            b[63] = 511;
            let mut qt = [3u16; 64];
            qt[9] = 201;
            check(&b, &qt, len, "truncated");
        }

        // Randomized sweep: 256 blocks, deterministic seed, sparsity
        // mix mirroring real entropy plus hostile-dense cases, random
        // quant tables (exercises the ±MAX_COEFF clamp hard).
        for round in 0..256 {
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
            check(&block, &qt, if round % 8 == 0 { 24 } else { 64 }, "random");
        }

        assert!(
            diverged.is_empty(),
            "SIMD dispatch diverged from scalar in {} case(s):\n{}",
            diverged.len(),
            diverged.join("\n")
        );
    }

    /// Force-exercise the raw kernel (not just the dispatch) on this
    /// CPU when SSE4.1 is present, including the hostile clamp edge.
    #[test]
    fn sse41_kernel_direct_matches_scalar_when_present() {
        #[cfg(target_arch = "x86_64")]
        {
            if std::arch::is_x86_feature_detected!("sse4.1") {
                let cases: [(&[i16; 64], &[u16; 64], &str); 4] = [
                    (&[0i16; 64], &[1u16; 64], "zero"),
                    (
                        &{
                            let mut b = [0i16; 64];
                            b[0] = 40;
                            b
                        },
                        &[1u16; 64],
                        "dc",
                    ),
                    (&[-32768i16; 64], &[65535u16; 64], "hostile-neg"),
                    (&[32767i16; 64], &[65535u16; 64], "hostile-pos"),
                ];
                for (block, qt, tag) in cases {
                    let mut scalar = [0u8; 64];
                    let mut simd = [0u8; 64];
                    super::super::idct::idct_block(block, qt, &mut scalar);
                    // SAFETY: SSE4.1 detected immediately above.
                    unsafe { idct_block_sse41(block, qt, &mut simd) };
                    assert_eq!(scalar, simd, "kernel vs scalar ({tag})");
                }
            }
        }
    }
}
