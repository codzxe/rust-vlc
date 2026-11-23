# AUDIT: crates/audio-engine/src/convert.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-23 |
| Lines / functions | 582 / 13 prod (incl. 2 macros) + 11 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:51 `use audio_engine::{AudioPipeline, PipelineSpec}` → crates/audio-engine/src/pipeline.rs:298 `frame_to_f32_planes(frame, &mut planes)?` and :425/:434 `planes_to_f32_interleaved_into` (the CLI audio limb; rc-smoke "command-layer-play" passes per baseline). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `cargo test -p audio-engine --lib convert` → 11 passed / 0 failed (exit 0). Zero-divisor semantics pinned experimentally: `4usize.is_multiple_of(0)==false`, `0usize.is_multiple_of(0)==true` (rustc 1.99.1, the pinned toolchain, scratch binary from stdin). |

## 1. One-paragraph verdict
This file is sample-FORMAT conversion only (any layout/sample → f32 planar; f32/s16 interleaved out) — **it contains no sample-rate conversion whatsoever** (the hostile-focus question is answered: there is none here to be correct or wrong; frame counts are conserved by every function, verified by read + tests, and the pipeline refuses rate mismatches upstream at pipeline.rs:288-289 `audio.rate_mismatch`; the only resampler in the crate is the ±1000 ppm clock-drift `MicroResampler` in resample.rs, which is NOT a general rate converter). The arithmetic that IS here is correct and unusually well-tested: monomorphic per-format loops, exact power-of-two reciprocals, s24 sign extension right, saturating s16 inverse with NaN→0, a full-range 65536-value s16 round-trip, and a real differential pinning the s16-stereo fast path bit-identical to the generic walk. Defects: (1) a latent divide-by-zero panic on zero-channel frames in its own error path — the sibling `frames_in` in media-types guards this, this copy forgot; (2) the "every int sample is exact in f32" doc claim is mathematically false for S32; (3) the formats are documented "native endian" in media_types but hard-coded little-endian here.

## 2. What actually works (verified)
— `cargo test -p audio-engine --lib convert` → 11/11 passed, incl. `s16_round_trip_is_exact_over_the_full_range` (all 65536 i16 values decode→encode identity) and `s16_stereo_fast_path_is_bit_identical_to_the_generic_loop` (xorshift content over 6 lengths).
— Bounds safety of the macro walks verified by read: last accessed byte = `c*bps + (frames-1)*frame_bytes + bps - 1 = data.len()-1`, guaranteed by the whole-frame multiple check at :151.

## 3. Findings
F-001 [S3]
Where: crates/audio-engine/src/convert.rs:150-157
Code:
```rust
    let frame_bytes = fmt.frame_bytes() as usize;
    if !data.len().is_multiple_of(frame_bytes) {
        return Err(error::partial_frame(
            frame_bytes as u64,
            (data.len() % frame_bytes) as u64,
        ));
    }
    let frames = data.len() / frame_bytes;
```
What is wrong: a zero-channel frame (`frame_bytes == 0`) panics in BOTH shapes: non-empty data → `is_multiple_of(0)` is `false` (verified on the pinned toolchain) → the error constructor itself evaluates `data.len() % 0` → modulo-by-zero panic at :154; empty data → `0.is_multiple_of(0)` is `true` → `0 / 0` divide-by-zero panic at :157. The sibling function this logic was cloned from guards it — media-types/src/frame.rs:77-82: `if frame == 0 || !(bytes as u64).is_multiple_of(frame) { return None; }`. Currently unreachable through the pipeline (pipeline.rs:114 refuses `spec.channels == 0`, :292 requires frame channels == mixer channels), and decoders refuse zero channels — but this is a `pub` API whose own typed-error path self-panics; the guard is two layers away and incidental.
Why it matters: any future producer of a zero-channel frame (or a direct API consumer) gets a panic exactly where the code promises a typed `audio.partial_frame`. What correct looks like: the same `frame_bytes == 0` early refusal `frames_in` has.
Confidence: HIGH — Verification: read + executed zero-divisor semantics probe (rustc 1.99.1).

F-002 [S3]
Where: crates/audio-engine/src/convert.rs:7-11 (doc) vs :64-97, :236, :274 (code)
Code:
```rust
//! corpus's `[audio_probe]` provenance uses — every int sample is
//! exact in f32, so the conversion is bit-deterministic on every
//! host.
```
What is wrong: "every int sample is exact in f32" is FALSE for S32: f32 has a 24-bit significand, so integers with magnitude > 2^24 (the entire upper half of the s32 range) round — e.g. s32 max `2_147_483_647` becomes `2147483648.0f32` (error half an LSB; `S32_RECIP` then maps it to exactly 1.0 instead of 0.99999999953). True for u8/s16/s24 (all fit in 24 bits); the full-range round-trip test that would expose it covers only s16. The conversion is still bit-deterministic — but "exact" is a false claim, and s24/s32 round-trips are untested.
Confidence: HIGH — Verification: read + arithmetic (f32 mantissa bound); no s32 round-trip test exists to run.

F-003 [S3]
Where: crates/audio-engine/src/convert.rs:64-97 vs crates/media-types/src/frame.rs:18-24
Code:
```rust
    /// Signed 16-bit, native endian.
    S16,
```
vs
```rust
fn s16_at(data: &[u8], i: usize) -> f32 {
    let v = i16::from_le_bytes([data[i], data[i + 1]]);
```
What is wrong: the format contract says "native endian" (S16/S24/S32 all documented so in media-types) while this file decodes and encodes strictly little-endian (`from_le_bytes`/`to_le_bytes` throughout, including the f32 device output). Masked on every current target (x86_64/aarch64/wasm32 are LE), but the documented contract and the implementation disagree — on a BE host every int path and the f32 device interleave would be byte-swapped garbage. One of the two sides is wrong on paper; the code is the de-facto contract.
Confidence: HIGH — Verification: read of both files.

F-004 [S4]
Where: crates/audio-engine/src/convert.rs:186-215
What is wrong: planar S24/S32/F32/F64 instantiations have zero test coverage (planar is tested for S16 only; `every_format_decodes_a_known_scalar` covers interleaved only). The macro makes them near-certainly right, but nothing pins the planar non-s16 arms.
Confidence: HIGH — Verification: read of test module; `cargo test -p audio-engine --lib convert` (no such tests exist).

F-005 [S4]
Where: crates/audio-engine/src/convert.rs:150
What is wrong: `fmt.frame_bytes() as usize` — u64→usize truncating cast; on wasm32 a hostile channel count × bps ≥ 2^32 would truncate `frame_bytes` and corrupt the whole-frame check. Unreachable via real containers (stsd/WAV channel fields are u16), pure cast hygiene.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `check_planes` | 44-53 | REAL | — | count + ragged detection, tested |
| `u8_at` | 59-61 | REAL | — | (v-128)·2^-7, tested |
| `s16_at` | 64-67 | REAL | S3 | LE vs "native endian" contract (F-003); exact |
| `s24_at` | 70-79 | REAL | S3 | sign extension correct (<<8 >>8); LE assumption (F-003); tested incl. -1.0 and max |
| `s32_at` | 82-85 | REAL | S3 | F-002 (not exact in f32), F-003 |
| `f32_at` | 88-90 | REAL | — | LE passthrough, round-trip tested |
| `f64_at` | 93-97 | REAL | — | narrowing, tested |
| `decode_interleaved!` | 106-114 | REAL | — | bounds-safe by construction (see §2) |
| `decode_planar!` | 119-128 | REAL | S4 | non-s16 arms untested (F-004) |
| `frame_to_f32_planes` | 143-218 | REAL | S3 | zero-channel panic (F-001); allocation-reuse claim machine-checked; s16-stereo fast path pinned bit-identical |
| `planes_to_f32_interleaved` | 226-240 | REAL | — | exact reserve, tested |
| `planes_to_f32_interleaved_into` | 256-278 | REAL | — | byte-identical to Vec version (differential test), wrong-size typed |
| `f32_to_s16_clamped` | 283-298 | REAL | — | saturate + NaN→0, all boundaries tested |
| `planes_to_s16_interleaved` | 306-320 | REAL | — | full-range round trip tested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| plane_capacity_is_reused_across_calls | YES | asserts concrete capacities survive geometry changes |
| interleave_into_is_byte_identical_to_the_vec_version | YES | byte equality + both wrong-size shapes typed |
| s16_interleaved_decodes_exactly | YES | exact f32 values incl. ±full scale |
| s16_planar_decodes_exactly | YES | exact planar split |
| every_format_decodes_a_known_scalar | YES | 7 scalar pins incl. s24 min/max, s32 min |
| partial_frames_are_typed_corrupt | YES | 3-byte stereo → `audio.partial_frame` |
| f32_interleave_round_trips_the_deinterleave | YES | byte equality |
| s16_saturates_at_the_boundary | YES | 8 boundary pins incl. ±inf, NaN |
| s16_round_trip_is_exact_over_the_full_range | YES | all 65536 values — would catch any scaling/rounding error |
| interleaved_output_rejects_ragged_planes | YES | both sinks typed |
| s16_stereo_fast_path_is_bit_identical_to_the_generic_loop | YES | adversarial lengths 0..4096 vs hand-written reference |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "any decoded layout (interleaved or planar; u8/s16/s24/s32/f32/f64) becomes f32 planar planes" | :4-6 | REAL | all 12 match arms present; interleaved all-formats + planar s16 executed green |
| "every int sample is exact in f32" | :9-10 | FALSE | false for S32 (24-bit f32 significand) — F-002 |
| "the exact algebraic inverse ... proven by the full-range round-trip test" | :17-19 | REAL | for s16 (the only int inverse shipped); test executed |
| "Frame counts are conserved through every function here" | :24-25 | REAL | read-verified every path; no rate logic exists in this file at all |
| "no per-frame allocation after warm-up" | :133-137 | REAL | capacity-reuse test executed |
| "v0 devices speak f32; s16 is the tested integer seam" | :14-16 | REAL-UNPROVEN | s16 seam tested here; whether any shipped device consumes s16 is outside this file (UNVERIFIED) |

## 7. Dead / duplicated / unreachable code
None. Every function has a caller (pipeline.rs:32 imports the two used; `planes_to_f32_interleaved`/`planes_to_s16_interleaved` are the Vec-shaped twins used by tests/other seams — pub API, minor duplication by design, documented as byte-identical and differentially pinned).

## 8. Dependencies & flags
No external deps; media-types + crate::error only. No unsafe, no features.

## 9. Open questions for the next agent
- resample.rs (another agent's file): confirm the micro-resampler is drift-only (±1000 ppm) and that NO general resampler exists anywhere — which means the AAC decoder.rs F-001 (implicit HE-AAC decoded at core rate) surfaces downstream as `audio.rate_mismatch` per frame (audio stall) or a wrongly-negotiated mixer rate, never as correct audio. convert.rs cannot mask it.
- Who negotiates `PipelineSpec.rate` (container-declared vs decoder-declared)? That decides which failure shape F-001 produces.
