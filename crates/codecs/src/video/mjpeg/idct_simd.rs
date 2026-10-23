//! SIMD plan for the MJPEG decode hot stages (card 7-b) — STUB, not
//! yet wired. The ONE sanctioned `unsafe` exception in the repo
//! (contract `.agent-brief-w7.md`, D-10): `std::arch` x86_64
//! intrinsics, runtime-dispatched via `is_x86_feature_detected!`,
//! every `unsafe` block carrying a safety comment, and a scalar
//! fallback that keeps non-x86_64 targets and pre-SSE4.1 CPUs
//! bit-exact (cfg-guarded, not assumed).
//!
//! # Why IDCT first (pending [VERIFY] from `stage_bench`)
//!
//! The pre-SIMD raw data (`docs/perf/raw/mjpeg/speed-before-avi-video.md`,
//! W7-b predecessor) puts whole-decode lumen at **0.30× ffmpeg** on
//! avi-video (medians, 5/8 files). ffmpeg's MJPEG path vectorizes the
//! IDCT and the YUV→RGB conversion; our stages are scalar. The
//! expected attribution from the code shape (and the W6-03 color_bench
//! raw data showing the color stage matters) is roughly
//! huffman ≈ idct ≈ color, all ahead of parse/store — but the numbers
//! MUST come from `stage_bench` (the ablation instrument committed in
//! this wave: `cargo run --release -p codecs --example stage_bench --
//! <heavy.mjpeg> [--passes N]`) before any SIMD lands. If that
//! ordering is wrong, SIMD the actual top stage instead.
//!
//! # Bit-exactness discipline (non-negotiable)
//!
//! `idct.rs` is integer-only, no floats, round-half-away-from-zero,
//! clip 0..=255 — deterministic across platforms, pinned by
//! `factored_idct_is_bit_exact_vs_naive` and the committed reference
//! digests. Any SIMD variant must produce **identical bytes**:
//! - keep every product in exactly the same width as the scalar path
//!   (`i32` MACs via `_mm_madd_epi16` on the Q16 table rows; the
//!   `i64` wide form the scalar path uses in pass 1 must be provably
//!   non-overflowing in `i32` first — `i16` sample × `i16` Q16
//!   coefficient pairs summed by `pmaddwd` stays in `i32` iff the
//!   intermediate bound holds; verify against the scalar saturating
//!   math over randomized blocks),
//! - replicate round-half-away-from-zero (not banker's rounding) and
//!   the asymmetric clip,
//! - pin equality with a randomized-block test (scalar vs SIMD) plus
//!   the existing end-to-end reference digests, and a
//!   `debug_assert`-free production path identical on both arms.
//!
//! # Dispatch skeleton (to be filled in)
//!
//! ```ignore
//! pub(crate) fn idct_block_dispatch(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
//!     #[cfg(target_arch = "x86_64")]
//!     {
//!         // SAFETY comment: `is_x86_feature_detected!` gates every
//!         // intrinsic block; the detected-feature closures contain
//!         // only `std::arch` intrinsics valid for that feature bit.
//!         if is_x86_feature_detected!("sse4.1") {
//!             return unsafe { idct_block_sse41(block, qt, out) };
//!         }
//!     }
//!     idct::idct_block(block, qt, out) // scalar fallback (bit-exact ref)
//! }
//! ```
//!
//! Wire-in point: `decoder.rs` `ScanHooks::transform` (the W7-b seam)
//! — swap the `idct_block` forwarding for `idct_block_dispatch`;
//! `stage_bench`'s `NoIdctStages` arm is unaffected, and the seam's
//! bit-exactness test then covers the dispatched path on this CPU.
//!
//! Same recipe for the color stage (`color.rs`, YUV→RGBA compose)
//! if `stage_bench` puts it on top instead: 8-wide `pshufb` planar
//! interleave + `pmaddubsw`-style fixed-point chroma lift, again
//! round-for-round identical to the scalar composer.
//!
//! # Measurement protocol (honest numbers)
//!
//! Before/after under the IDENTICAL 20261022 protocol: same corpus,
//! same N/passes, medians + p95, `/proc/loadavg` sampled around the
//! window, raw JSON committed to `docs/perf/raw/mjpeg/`, the speed
//! doc updated with the delta vs the 0.30× baseline — no
//! cherry-picking, unmeasured claims marked [VERIFY].
