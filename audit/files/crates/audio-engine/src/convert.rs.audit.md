# AUDIT: crates/audio-engine/src/convert.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 583 / 12 functions + 2 macros + 12 tests |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — `lumen` bin → run.rs open → apps/cli/src/host.rs `open_source` → apps/cli/src/pipeline_host.rs `open_audio_session` (AudioLimb wraps audio-engine `AudioPipeline`) → crates/audio-engine/src/pipeline.rs:298 `frame_to_f32_planes(frame, &mut planes)` and pipeline.rs:425/434 `planes_to_f32_interleaved_into` (the quantum hot path). `planes_to_s16_interleaved`/`planes_to_f32_interleaved` (Vec forms) have no production caller found (tests only) — see F-003. |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:2 |
| Commands run | `sed -n` quote verifications (convert.rs 44-53, 147-157, 226-233; pipeline.rs 294-300; media-types/frame.rs 28-80) — exit 0; `rg` consumer searches. No cargo (adaptation). |

## 1. One-paragraph verdict

This is real, careful sample-format conversion code — the best-audited kind in this wave. Stage A (any u8/s16/s24/s32/f32/f64 × interleaved/planar → f32 planar) uses exact power-of-two reciprocals (bit-identical to division), sign-extends s24 correctly, validates whole-frame boundaries with a typed error, and reuses plane allocations across calls. Stage D (f32/s16 interleaved) has a saturating NaN-safe clamp and an exact full-range i16 round-trip test. No stubs, no swallowed errors, no fabricated values. Defects found: a division-by-zero panic if a zero-channel AudioFormat ever reaches the entry (guarded today only by upstream demuxer validation, not locally), a tautological plane-count check that looks like validation but validates nothing, and two Vec-based output functions with no production caller.

## 2. What actually works (verified)

- int→f32 scaling: `S16_RECIP = 1.0/32_768.0` etc. are exact powers of two, exactly representable in f32 — the multiply is bit-identical to the divide. Read-verified (convert.rs:38-41); pinned by `every_format_decodes_a_known_scalar` (441-468) with exact expected floats (e.g. `8_388_607.0 / 8_388_608.0`).
- s24 sign extension (70-79): `((v24 << 8) as i32) >> 8` arithmetic-shift sign-extend — hand-checked 0x800000 → −1.0, 0x7FFFFF → 8388607/8388608; both pinned by test.
- Interleave deinterleave bounds: after the whole-frame check (151), macro indices stay within `frames·frame_bytes`; no indexing panic for any well-formed payload; ragged planes rejected typed (`interleaved_output_rejects_ragged_planes`).
- `f32_to_s16_clamped` (283-298): NaN→0 documented, ±∞ and out-of-range saturate — pinned by `s16_saturates_at_the_boundary` (501-510); full-range round-trip identity over all 65536 i16 values (`s16_round_trip_is_exact_over_the_full_range`, 513-532).
- Allocation reuse: `resize_with` + per-plane `resize` keep inner Vecs alive — machine-checked by `plane_capacity_is_reused_across_calls` (352-377).
- W00 baseline (cited): release build PASS; mjpeg+pcm decodes in decode-bench — the PCM path exercises this code and produces audio samples.

## 3. Findings

### F-001 [S3] Zero-channel `AudioFormat` panics on division by zero (`is_multiple_of(0)`)
Where: crates/audio-engine/src/convert.rs:150-155
Code (verbatim):
```rust
    let frame_bytes = fmt.frame_bytes() as usize;
    if !data.len().is_multiple_of(frame_bytes) {
        return Err(error::partial_frame(
            frame_bytes as u64,
            (data.len() % frame_bytes) as u64,
        ));
    }
```
What is wrong: `frame_bytes()` is `channels × bytes_per_sample` (media-types/src/frame.rs:70-72); `channels == 0` gives `frame_bytes == 0`, and `usize::is_multiple_of(0)` evaluates `len % 0` — an unconditional panic (all profiles). media-types itself treats zero frame bytes as representable (`frames_in` guards `frame == 0` at frame.rs:79-80), so the ecosystem acknowledges the shape; convert.rs does not guard it. Today no shipped demuxer produces a 0-channel format (verified: demux/src/wav/format.rs:355 `if channels == 0`, aiff/chunks.rs:259, avi/header.rs:826, asf/header.rs:798, adts/mod.rs:590 all reject), so the panic is not currently reachable from a malformed file — the defense is incidental and distributed, not local to the function that crashes.
Why it matters: any future decoder or plugin host constructing an `AudioFormat { channels: 0, .. }` frame crashes the engine with a division panic instead of the typed `Corrupt` error this module otherwise champions.
What correct looks like: `if frame_bytes == 0 || !data.len().is_multiple_of(frame_bytes) { return Err(...) }` — one clause, matching `frames_in`'s existing discipline.
Confidence: HIGH — Verification: read (Rust `%` semantics); reachability of channels=0: no producer found (rg over demux/src).
Severity note: not S1 because no reachable producer exists in the shipped build; rated S3 as a latent panic on the module's own pub API.

### F-002 [S4] `check_planes`' plane-count check is a tautology at every call site
Where: crates/audio-engine/src/convert.rs:44-47, 230-231
Code (44-47, verbatim):
```rust
fn check_planes(planes: &[Vec<f32>], channels: u32) -> Result<usize, EngineError> {
    if planes.len() != channels as usize {
        return Err(error::plane_mismatch("audio.planar_plane_mismatch"));
    }
```
Code (226-231, verbatim):
```rust
pub fn planes_to_f32_interleaved(
    planes: &[Vec<f32>],
    out: &mut Vec<u8>,
) -> Result<(), EngineError> {
    let channels = planes.len() as u32;
    let frames = check_planes(planes, channels)?;
```
What is wrong: every caller derives `channels` from `planes.len()` itself, so the first check can never fire — dead validation that reads as a real invariant. The meaningful check (equal plane lengths, 48-51) survives. The parameter and the error branch are misleading scaffolding.
What correct looks like: drop the `channels` parameter (check only geometry), or pass the *expected* channel count from the format.
Confidence: HIGH — Verification: read; all three call sites (231, 261, 311) derive the argument the same way.

### F-003 [S4] Two Vec-based output functions have no production caller
Where: crates/audio-engine/src/convert.rs:226-240 (`planes_to_f32_interleaved`), 306-320 (`planes_to_s16_interleaved`)
What is wrong: rg over crates/ and apps/ shows the only callers are this file's own tests (386, 524, 540) and the doc-comment cross-references. The production quantum path uses `planes_to_f32_interleaved_into` (pipeline.rs:425/434); the s16 target is the "tested integer seam for the backends later cards add" (module doc 15-17) — i.e., a seam for devices that do not exist yet. Honest per the doc, but it is unused shipped surface.
Confidence: HIGH — Verification: rg caller search.
Note: the doc at 14-17 admits v0 devices speak f32; the s16 seam is future work — PAPER-only-by-admission, not fraud.

### F-004 [S3] Fast-path duplication risk is managed, but the differential proof is self-referential
Where: crates/audio-engine/src/convert.rs:169-185 and test 545-581
Code (174-183, verbatim):
```rust
    if fmt.layout == AudioLayout::Interleaved
        && fmt.sample == SampleFormat::S16
        && fmt.channels == 2
    {
        let (quads, _) = data.as_chunks::<4>();
        for (f, quad) in quads.iter().enumerate() {
            let [l0, l1, r0, r1] = *quad;
            planes[0][f] = f32::from(i16::from_le_bytes([l0, l1])) * S16_RECIP;
            planes[1][f] = f32::from(i16::from_le_bytes([r0, r1])) * S16_RECIP;
```
What is wrong: nothing functionally — the differential test (545-581) proves bit-identity over adversarial lengths with a hand-rolled reference that re-implements the same arithmetic. That is the correct shape for a fast-path pin, and the arithmetic is identical by construction. The residual risk is ordinary duplication (two encodings of the s16→f32 step that must be edited in lockstep); the test bounds it. Listed because §3.1 requires copy-paste blocks to be named. Severity is the floor of the scale for a guarded duplication.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `check_planes` | 44-53 | REAL | S4 | count check tautological (F-002); length check real |
| `u8_at` | 58-61 | REAL | — | 128-offset, exact |
| `s16_at` | 63-67 | REAL | — | |
| `s24_at` | 69-79 | REAL | — | correct sign extension |
| `s32_at` | 81-85 | REAL | — | |
| `f32_at` | 87-90 | REAL | — | LE passthrough |
| `f64_at` | 92-97 | REAL | — | narrowing cast, documented |
| `decode_interleaved!` | 106-114 | REAL | — | zip-bounded, no per-sample branch; in-bounds after whole-frame check |
| `decode_planar!` | 119-128 | REAL | — | contiguous segs, exact bounds |
| `frame_to_f32_planes` | 143-218 | REAL | S3 | zero-frame_bytes panic corner (F-001); allocation reuse real; s16-stereo fast path (F-004) |
| `planes_to_f32_interleaved` | 226-240 | REAL | S4 | no production caller (F-003) |
| `planes_to_f32_interleaved_into` | 256-278 | REAL | — | exact-length typed check; chunks_exact_mut + as_chunks_mut; production quantum path |
| `f32_to_s16_clamped` | 283-298 | REAL | — | NaN→0, saturating, exact inverse |
| `planes_to_s16_interleaved` | 306-320 | REAL | S4 | future-device seam, no production caller (F-003) |
| tests (12) | 322-582 | REAL | — | see §5 |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| plane_capacity_is_reused_across_calls | YES (reasoned) | asserts capacities survive geometry change — catches a `clear()`-style regression |
| interleave_into_is_byte_identical_to_the_vec_version | YES (reasoned) | byte equality + both off-by-one buffer sizes rejected typed |
| s16_interleaved_decodes_exactly | YES (reasoned) | exact float vectors incl. ±full-scale |
| s16_planar_decodes_exactly | YES (reasoned) | |
| every_format_decodes_a_known_scalar | YES (reasoned) | one exact scalar per format incl. s24 sign corner |
| partial_frames_are_typed_corrupt | YES (reasoned) | 3-byte stereo → typed error, message checked |
| f32_interleave_round_trips_the_deinterleave | YES (reasoned) | byte equality with source |
| s16_saturates_at_the_boundary | YES (reasoned) | ±1, ±2, ±∞, NaN, 0 |
| s16_round_trip_is_exact_over_the_full_range | YES (reasoned) | all 65536 values, identity assert |
| interleaved_output_rejects_ragged_planes | YES (reasoned) | both output targets |
| s16_stereo_fast_path_is_bit_identical_to_the_generic_loop | YES (reasoned) | xorshift content, lengths {0,1,2,3,1470,4096} |
| (capacity test) | YES (reasoned) | see above |

No tautologies; no mock-of-subject; none `#[ignore]`. Mutation run not possible (no cargo) — reasoned only, UNVERIFIED at runtime.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every int sample is exact in f32, so the conversion is bit-deterministic" | convert.rs:7-11 | REAL | power-of-two reciprocals read-verified; s16 path ≤ 2^15 exact in f32 |
| "v0 devices speak f32 … the s16 target exists as the tested integer seam" | convert.rs:14-17 | REAL (and an admission) | s16 functions test-only (F-003) |
| "no per-frame allocation after warm-up" | convert.rs:132-137 | REAL-UNPROVEN | resize_with/resize logic read-verified + capacity test; not profiled (no cargo) |
| "Frame counts are conserved through every function" | convert.rs:24-25 | REAL | every path returns/uses the same `frames` value; no truncation path found |

## 7. Dead / duplicated / unreachable code

- `planes_to_f32_interleaved`, `planes_to_s16_interleaved` — no production callers (F-003).
- `check_planes`' first branch — unreachable at all call sites (F-002).

## 8. Dependencies & flags

- Depends only on media-types (frame types) and crate::error — consistent with the L2/L3 boundary. No feature gates. Compiled into every build linking audio-engine (W00: release PASS).

## 9. Open questions for the next agent

- Confirm no plugin/wasm host constructs `AudioFormat` with `channels: 0` (plugin-host-wasm, plugin-api) — F-001's panic depends on that never happening.
- The mixer's `map.apply(&planes, &mut staged)` (pipeline.rs:300) consumes these planes — verify the mixer's channel-count assumptions match (audio-engine/src/mixer.rs, another agent).
