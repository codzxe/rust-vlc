# AUDIT: crates/audio-engine/src/convert.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 582 / 14 prod items (+4 consts) + 12 tests + 1 test helper |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/pipeline_host.rs:660` `AudioPipeline::process` → `crates/audio-engine/src/pipeline.rs:298` `frame_to_f32_planes` (convert.rs:143); `pipeline_host.rs:661` `mix()` → `pipeline.rs:425/434` `planes_to_f32_interleaved_into` (convert.rs:256). apps/web/src/host.rs:74 uses the same crate.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n 'frame_to_f32_planes\|planes_to_f32_interleaved\|planes_to_s16_interleaved\|f32_to_s16_clamped' crates apps` (callers: only pipeline.rs `_into` variants — Vec/s16 versions test-only); `rg -n 'audio-engine' apps/*/Cargo.toml` (cli:48, web:67); `sed -n` quote verification; `rg -n 's16' crates/audio-output/src/null.rs` (null backend refuses s16 at :654-660) |

## 1. One-paragraph verdict
Genuine, correct sample-format conversion core: int→f32 power-of-two scaling, LE decode of all six formats, both layouts, exact s16 round-trip proven by a full-range test, and a bit-identical differential test pinning the stereo fast path. It is reachable and executed in the shipped AVI/PCM first-vertical lane (baseline `rc-smoke-engine-only.log` 6/6; `avi-firstvertical-open-session.txt` reaches Playing with the audio limb wired at pipeline_host.rs:660-662). Defects are bounded: three of its five public functions are production-dead (test-only callers), a 0-channel frame hits unguarded `is_multiple_of(0)`/division-by-zero, a validation parameter is tautological, and two doc claims overstate exactness and contradict the format-type's endianness contract.

## 2. What actually works (verified)
— verified by call-chain trace (pipeline_host.rs:660→pipeline.rs:298→convert.rs:143; pipeline_host.rs:661→pipeline.rs:425/434→convert.rs:256) + baseline `audit/runs/baseline/INDEX.md` rc-smoke PASS (AVI first-vertical plays end to end; audio limb runs into the null device) → conversion executed in the shipped play lane.
— verified by reading the 12 in-file tests: full-i16-range round trip (`s16_round_trip_is_exact_over_the_full_range`, :513-532), per-format scalar decode (`every_format_decodes_a_known_scalar`, :441-468), differential fast-path bit-identity (:545-581), ragged-plane rejection (:535-542), partial-frame typed error (:471-483). Arithmetic checked by hand: u8 (v−128)/128, s24 sign-extension `((v24<<8) as i32)>>8`, saturating clamp bounds — all correct.

## 3. Findings
### F-001 [S3]
Where: crates/audio-engine/src/convert.rs:226-320 (planes_to_f32_interleaved, planes_to_s16_interleaved, f32_to_s16_clamped)
Code:
```
pub fn planes_to_s16_interleaved(
    planes: &[Vec<f32>],
    out: &mut Vec<u8>,
) -> Result<(), EngineError> {
```
What is wrong: Three of the five public functions have zero production callers. `rg` across `crates/` and `apps/` finds only `pipeline.rs:32` importing `frame_to_f32_planes` + `planes_to_f32_interleaved_into`; the Vec-based `planes_to_f32_interleaved`, `planes_to_s16_interleaved`, and their helper `f32_to_s16_clamped` are called exclusively from this file's tests. The "integer-device seam" has no device: the null backend refuses s16 (`crates/audio-output/src/null.rs:654-660` tests that s16 open is a typed error) and no other backend is shipped.
Why it matters: Dead public API presented as stage D capability; the lib.rs doc (audio-engine/src/lib.rs:20-23, "integer targets are converted with saturating clamp for the backends later cards add") reads as if s16 output is a functioning lane when no consumer exists in any shipped build.
What correct looks like: Either `#[cfg]`-gate the s16/Vec seams behind the card that adds a real consumer, or mark them `#[doc(hidden)]`/documented-unwired in the crate docs; keep the tested seam out of the public surface until a backend calls it.
Confidence: HIGH — Verification: read (rg callers) / ran(baseline null.rs behavior cited).

### F-002 [S3]
Where: crates/audio-engine/src/convert.rs:149-157
Code:
```
    let bps = fmt.sample.bytes_per_sample() as usize;
    let frame_bytes = fmt.frame_bytes() as usize;
    if !data.len().is_multiple_of(frame_bytes) {
        return Err(error::partial_frame(
            frame_bytes as u64,
            (data.len() % frame_bytes) as u64,
        ));
    }
    let frames = data.len() / frame_bytes;
```
What is wrong: `frame_bytes()` (media-types frame.rs:70-72) is `channels * bytes_per_sample`; a 0-channel frame yields `frame_bytes == 0`. `is_multiple_of(0)` semantics are not guarded here — unlike media-types' own `frames_in` (frame.rs:74-81) which explicitly special-cases `frame == 0`. If `is_multiple_of(0)` evaluates true for empty data (the `self == 0` convention), line 157 `data.len() / frame_bytes` is a divide-by-zero panic; if it evaluates false, an empty 0-channel frame takes the error path but a non-empty one silently mis-reports geometry. I could not execute rustc in this sandbox (no toolchain on PATH) to pin the exact semantic — both outcomes are unhandled defects.
Why it matters: The crate's own doc (lib.rs:51-52) claims "No panics, no unwraps"; the sibling API in media-types demonstrates the author knew zero-frame-bytes needed a guard and omitted it here. Blast radius is limited: the only production caller (pipeline.rs:289-295 `process`) rejects `frame.format.channels != map.in_channels()` first, so this needs a `PipelineSpec` built with a 0-channel track map.
What correct looks like: Mirror `frames_in`: `if frame_bytes == 0 { return Err(error::plane_mismatch(...)) }` (or type a 0-channel frame as `Corrupt`) before the divisibility check.
Confidence: MEDIUM — Verification: read; panic semantics UNVERIFIED (no rustc in sandbox; REQUEST-ORCH-RUN noted in log).

### F-003 [S4]
Where: crates/audio-engine/src/convert.rs:44-53 (call sites 231, 261, 311)
Code:
```
fn check_planes(planes: &[Vec<f32>], channels: u32) -> Result<usize, EngineError> {
    if planes.len() != channels as usize {
        return Err(error::plane_mismatch("audio.planar_plane_mismatch"));
    }
```
What is wrong: Every call site passes `channels = planes.len() as u32`, so the first branch is a tautology that can never fire — a validation that validates nothing, with a parameter that implies an independent check exists.
Why it matters: Misleading API: a reader (or future caller passing a real channel count) believes plane-count-vs-channel-count is enforced at this seam; it is only enforced by accident of the call sites.
What correct looks like: Drop the `channels` parameter and the dead branch (the ragged-length check below it is the real invariant), or have callers pass the negotiated channel count from the spec instead of `planes.len()`.
Confidence: HIGH — Verification: read (all three call sites inspected).

### F-004 [S4]
Where: crates/audio-engine/src/convert.rs:7-11 and 65 (vs crates/media-types/src/frame.rs:16-29)
Code:
```
//! divide (`v / 2^(bits-1)`), the same pinned conversion the fixture
//! corpus's `[audio_probe]` provenance uses — every int sample is
//! exact in f32, so the conversion is bit-deterministic on every
//! host.
```
What is wrong: Two doc-precision defects. (a) "every int sample is exact in f32" is false for S32: f32 has a 24-bit significand, so any |v| > 2^24 rounds (e.g. 2^24+1). The conversion is *deterministic*, not *exact*, for s32. (b) convert.rs decodes S16/S24/S32 unconditionally little-endian (`i16::from_le_bytes`, line 65; comment "Little-endian 24-bit", line 74) while the type it decodes is documented "Signed 16-bit, **native endian**" (media-types frame.rs:20, and S24/S32 at :22/:24). The two crates disagree on the byte-order contract; they coincide only because every shipped target is little-endian and the PCM decoder emits native/LE bytes (pcm/decoder.rs:736-741 passthrough).
Why it matters: The doc claims a mathematical property the code does not have, and the endianness contract is only accidentally consistent — a big-endian target (none shipped today) or a future BE-source producer would silently corrupt audio while both docs claim correctness.
What correct looks like: Fix the module doc to "exact for u8/s16/s24, deterministic round-to-nearest for s32"; fix media-types' SampleFormat docs to say little-endian (or make convert.rs honor native endianness and document LE-only producers) — one consistent contract.
Confidence: HIGH — Verification: read (both files; f32 mantissa width is IEEE-754 fact).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| U8/S16/S24/S32_RECIP consts | 38-41 | REAL | — | Exact power-of-two reciprocals; correct. |
| check_planes | 44-53 | REAL | S4 | Ragged-length check real; channel-count branch tautological (F-003); empty planes → frames=0 Ok. |
| u8_at | 58-61 | REAL | — | Correct (v−128)/128. |
| s16_at | 63-67 | REAL | — | Correct LE i16 (endianness contract defect noted in F-004b). |
| s24_at | 69-79 | REAL | — | Correct LE sign-extended 24-bit; hand-verified 0x800000→−1.0. |
| s32_at | 81-85 | REAL | — | Correct; f32 rounding for \|v\|>2^24 inherent (F-004a). |
| f32_at | 87-90 | REAL | — | LE passthrough, correct. |
| f64_at | 92-97 | REAL | — | Correct narrowing copy. |
| decode_interleaved! | 106-114 | REAL | — | Zip bounds at frames; max index = data.len(); no overflow; `$frames` param unused (zip bounds it — documented). |
| decode_planar! | 119-128 | REAL | — | Segments tile data.len() exactly (frames derived from same divisor); safe. |
| frame_to_f32_planes | 143-218 | REAL | S3 | Hot path, reachable, allocation-reuse claim verified by test; 0-channel guard missing (F-002); stereo-s16 fast path bit-identical (differential test :545-581). |
| planes_to_f32_interleaved | 226-240 | UNREACHABLE (prod) | S3 | Test-only callers (F-001); correct if called. |
| planes_to_f32_interleaved_into | 256-278 | REAL | — | Production hot path; exact-length typed check; chunks_exact_mut bounds correct. |
| f32_to_s16_clamped | 283-298 | UNREACHABLE (prod) | S3 | NaN→0 documented; clamp bounds correct; test-only caller (F-001). |
| planes_to_s16_interleaved | 306-320 | UNREACHABLE (prod) | S3 | Test-only callers; null device refuses s16 (F-001). |
| tests: frame_of | 331-349 | helper | — | — |
| plane_capacity_is_reused_across_calls | 351-377 | REAL test | — | Would fail under `clear()`-style churn. |
| interleave_into_is_byte_identical… | 379-401 | REAL test | — | Differential + both wrong-length branches asserted. |
| s16_interleaved_decodes_exactly | 403-420 | REAL test | — | Exact-value assertions incl. −32768→−1.0. |
| s16_planar_decodes_exactly | 422-438 | REAL test | — | Exercises decode_planar!. |
| every_format_decodes_a_known_scalar | 440-468 | REAL test | — | All six formats; u8 255→127/128 asymmetry pinned. |
| partial_frames_are_typed_corrupt | 470-483 | REAL test | — | 3-byte stereo s16 → typed Corrupt. |
| f32_interleave_round_trips… | 485-498 | REAL test | — | f32 path identity. |
| s16_saturates_at_the_boundary | 500-510 | REAL test | — | ±∞, NaN, ±1.0 boundaries. |
| s16_round_trip_is_exact_over_the_full_range | 512-532 | REAL test | — | All 65536 i16 values; proves the exact-inverse claim for s16. |
| interleaved_output_rejects_ragged_planes | 534-542 | REAL test | — | Would fail if check_planes' second branch broke. |
| s16_stereo_fast_path_is_bit_identical… | 544-581 | REAL test | — | xorshift content over lengths {0,1,2,3,1470,4096}; genuine differential. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 12 tests | YES | Concrete mutation check: change `S16_RECIP` to `1.0/32767.0` → 5 value tests fail; swap fast-path L/R indices → differential test fails at frames≥1; remove clamp branch → `s16_saturates_at_the_boundary` fails; make check_planes always Ok → ragged-planes test fails. No tautologies, no mocks (tests exercise the real functions, `testkit::Must` only unwraps Results). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every int sample is exact in f32" | convert.rs:9-10 | FALSE (for s32) | IEEE-754 f32 24-bit significand; see F-004a. |
| "the exact algebraic inverse of stage A" (s16) | convert.rs:17-18, 280-282 | REAL | Full-range round-trip test :513-532 (also in baseline test-workspace run, 1681 passed). |
| "Frame counts are conserved through every function here" | convert.rs:24-25 | REAL | Every function maps frames 1:1; zip-bounded loops; verified by reading + round-trip tests. |
| Stage D integer seam "for the backends later cards add" | lib.rs:20-23, convert.rs:300-301 | SHALLOW | No backend consumer; null.rs:654-660 refuses s16 (F-001). |
| "no per-frame allocation after warm-up" | convert.rs:131-137 | REAL | resize_with/plane.resize retention; machine-checked by :351-377. |
| Endianness of S16/S24/S32 = "native endian" | media-types frame.rs:20-24 vs convert.rs:65,74 | FALSE (contract contradiction) | convert.rs hardcodes LE; see F-004b. |

## 7. Dead / duplicated / unreachable code
- `planes_to_f32_interleaved` (226-240), `planes_to_s16_interleaved` (306-320), `f32_to_s16_clamped` (283-298): production-dead (F-001).
- `check_planes` first branch (45-47): dead at all call sites (F-003).
- `decode_interleaved!`'s `$frames` / `decode_planar!`'s `$frames` macro params unused — benign (zip bounds), noted for hygiene only.

## 8. Dependencies & flags
- Depends only on `media_types` (frame, EngineError) + `crate::error` + dev-dep `testkit` — matches audio-engine/Cargo.toml (std-only, zero external deps). No feature gates. No `unsafe`. No `#[allow]`.
- Reachability independent of codec features: converts whatever decoders emit; exercised by the pcm_s16le AVI lane in the shipped build.

## 9. Open questions for the next agent
- pipeline.rs / mix.rs / channel_map.rs auditors: confirm `PipelineSpec` cannot be constructed with 0 channels (would upgrade F-002's blast radius).
- audio-output auditor: is any non-null backend (cpal) compiled in any shipped build, and does it consume f32 or s16 (resolves F-001's seam permanently)?
- media-types auditor: fix the "native endian" doc contradiction (F-004b) — the effective contract is little-endian.
