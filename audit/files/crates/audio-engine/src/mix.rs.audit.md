# AUDIT: crates/audio-engine/src/mix.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Lines / functions | 217 / 12 (Mixer: 5 methods + struct; 6 tests + 1 helper) |
| File verdict | REAL (core is live, correct, deterministic; one false safety-property doc claim + dead clamp code) |
| Reachable from shipped entry point? | YES for `mix` — call chain: `lumen open` → pipeline_host.rs AudioLimb → `AudioPipeline::mix` (pipeline.rs:343 `self.mixer.mix(&self.stage, &mut mixed)`) per quantum; executed on the AVI first-vertical lane (60/60 a_frames decoded, baseline mp4-smoke-bench-avi-mjpeg-pcm.txt) and sample-aac.mp4. `new`/`tracks` reachable via `AudioPipeline::new` (pipeline.rs:122). `set_volume`/`volume`: NO shipped caller (W07-A01 part1 finding — test/API-only). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read mix.rs 1-217 · hand re-derivation of all four arithmetic tests (11/22/44/88; 2.5; 7; unity) · `rg -n "f32_to_s16_clamped\|planes_to_s16_interleaved" crates` (callers: convert.rs's own tests ONLY — no production caller) · Read convert.rs 250-290 (stage-D F32 interleave = pure passthrough, no clamp) · `rg -n "chunk_alignment\|planar_plane_mismatch" crates/audio-engine/src/error.rs` (chunk_alignment → `audio.chunk_length_mismatch`, Corrupt{Timing}) · baseline test-workspace.log.txt:95-98 (4 mix tests green) · `sed -n` quote verification below |

## 1. One-paragraph verdict
The mixer core does exactly what it claims: ascending-track `*dst += sample * gain` over validated-aligned planar chunks, deterministic by construction, zero-initialized reused output planes (clear+resize keeps capacity — D-1 verified by `output_capacity_is_reused`), and every geometry violation is a typed error with no panic path I could construct (empty inputs, zero channels, zero frames, ragged planes, cross-track disagreement all land in typed errors or safe no-ops). All four arithmetic tests hand-check exactly. Two real defects: the module doc promises that "stage D's saturating clamp is where magnitude is bounded" — but the shipped stage D (`planes_to_f32_interleaved_into`, convert.rs:256-278) is a pure f32 passthrough with no clamp, and the clamp the doc must be thinking of (`f32_to_s16_clamped`/`planes_to_s16_interleaved`) has ZERO production callers (convert.rs's own tests only), so summed overflow flows to the sink unbounded and the "clamp" is dead code; and the `mix` doc's error taxonomy promises `audio.planar_plane_mismatch` for ragged planes while the code returns `audio.chunk_length_mismatch` for every geometry error.

## 2. What actually works (verified)
— Hand-checked arithmetic: two_tracks 1+10=11, 2+20=22, 4+40=44, 8+80=88; volumes 0.5·1+2.0·1=2.5; mute 5·MUTE+7=7; unity passthrough — all four tests' exact values reproduce.
— Geometry validation (99-105): channels/frames taken from track 0; every chunk's plane count and every plane's length cross-checked — ragged, channel-mismatch, and frame-mismatch all typed; `zip` in the sum loop is therefore always exact-length.
— Baseline `audit/runs/baseline/test-workspace.log.txt:95-98` — `mix::tests::{output_capacity_is_reused, per_track_volumes_apply_before_summing, single_track_at_unity_is_bit_exact, two_tracks_sum_in_track_order}` all ok; the mixer ran end-to-end on the AVI first-vertical bench (60/60 audio frames, INDEX.md) — the LIVE shipped path.
— Zero-frame / zero-channel safety: `channels=0` → empty out, `Ok(0)`; `frames=0` → planes resized to 0, `Ok(0)` (the pipeline gates `frames == 0` before calling, so this is defense-in-depth — traced).
— Output-plane shrink path: `resize_with` truncates extra planes only on a geometry change (cold path); per-plane `clear()+resize(frames)` preserves capacity on the warm path — the `output_capacity_is_reused` test pins exactly this.

## 3. Findings
### F-001 [S3]
Where: crates/audio-engine/src/mix.rs:11-14 (doc claim), with crates/audio-engine/src/convert.rs:256-278 (actual stage D) and convert.rs:283/306-318 (the dead clamp)
Code:
```
//! Sum overflow stays in the f32 domain (values may exceed ±1.0 or
//! grow toward infinity — *data* at this stage); stage D's saturating
//! clamp is where magnitude is bounded (doc 03 §4.8 e's soft limiter
//! is the later hardening).
```
What is wrong: there is no saturating clamp on the shipped stage-D path. The pipeline's stage D is `planes_to_f32_interleaved_into` (pipeline.rs:367 → convert.rs:256-278), which copies f32 samples to bytes verbatim — a mix output of +8.0, +inf, or NaN reaches the sink exactly as-is. The functions that DO clamp (`f32_to_s16_clamped`, `planes_to_s16_interleaved`, convert.rs:283-318, including documented NaN→0 and ±inf→rail behavior) have zero production callers — `rg` over `crates/` shows usage only inside convert.rs's own test module.
Why it matters: the doc states a magnitude bound that does not exist in the live path. Today the only sink is the null device (no audible consequence — the honest mitigating fact), but any future real output device wired to this pipeline inherits unbounded f32 the docs promised was clamped; and the tested, correct clamp code sits unreachable while the doc points at it.
What correct looks like: either wire the clamp into the device-adaptation stage for real backends (the S16 lane exists, dormant) or rewrite the doc: "v0's F32 device format is unclamped; magnitude bounding joins with real output backends (doc 03 §4.8 e)".
Confidence: HIGH — Verification: read of both files + caller exhaustion (`rg` in crates/); null-device-only consequence confirmed from pipeline_host.rs wiring + INDEX.md ("audible output: none — null device").

### F-002 [S4]
Where: crates/audio-engine/src/mix.rs:85-87 (doc) vs 102-103 (code)
Code:
```
    /// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
    /// `audio.planar_plane_mismatch` (a track's planes are ragged) or
```
```
        for chunk in inputs {
            if chunk.len() != channels || chunk.iter().any(|p| p.len() != frames) {
                return Err(error::chunk_alignment());
```
What is wrong: the doc promises `audio.planar_plane_mismatch` for ragged planes; the code returns `error::chunk_alignment()` for the ragged case (and for channel-count and frame-count disagreement) — which is `audio.chunk_length_mismatch` with `Corrupt { detail: Timing }` (error.rs:86-94). A ragged plane is a geometry error, not a timing error; the promised code exists in the crate (`plane_mismatch`, used by convert.rs's `check_planes`) but is never produced by `mix`.
Why it matters: callers switching on error codes (the pipeline maps these to user-facing decode errors) get `chunk_length_mismatch` where the API doc says `planar_plane_mismatch`; the taxonomy test in this very file (182-196) asserts only `chunk_length_mismatch` — the tests pin the code's behavior, the doc pins a different one, and the doc is the loser nobody noticed.
What correct looks like: return `error::plane_mismatch("audio.planar_plane_mismatch")` for the ragged arm (102-103), or fix the doc.
Confidence: HIGH — Verification: read of mix.rs + error.rs:86-94 + convert.rs:44-52 (where the promised code is actually used).

### F-003 [S4]
Where: crates/audio-engine/src/mix.rs:137-144 (the "bit exact" test)
Code:
```
    fn single_track_at_unity_is_bit_exact() {
        let mixer = Mixer::new(1).must();
        let input = planes(&[&[0.0, 0.5, -0.25, 1.0], &[0.125, -1.0, 0.75, -0.5]]);
        let mut out = Vec::new();
        let frames = mixer.mix(std::slice::from_ref(&input), &mut out).must();
        assert_eq!(frames, 4);
        assert_eq!(out, input);
```
What is wrong: the test named "bit exact" asserts with `PartialEq`, under which `-0.0 == 0.0` and NaN≠NaN — a mutation that flushes `-0.0` to `+0.0` (or quiets a NaN payload) passes. The input also contains no `-0.0`, no NaN, no ±inf, so the mix's behavior on special values is unpinned at this layer (the resampler's special-value discipline is bit-level via `to_bits()` — the mixer's is not).
Why it matters: the module doc (7-9) leans on "byte-stable across runs and hosts, which the sink-buffer hash AC depends on" — the local pin for that property cannot detect sign-bit or NaN-payload changes.
What correct looks like: compare `to_bits()` vectors and include `-0.0`/NaN/inf samples in the input (the pipeline's unity test covers ordinary values; special values are the gap).
Confidence: HIGH — Verification: read; mutation reasoning (PartialEq semantics).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-14 | REAL | F-001, F-002 | determinism claim correct (fixed order); "stage D clamp" claim false (F-001) |
| `Mixer` struct | 22-25 | REAL | — | volumes only — stateless mix parameters |
| `Mixer::new` | 35-42 | REAL | — | refuses 0 tracks typed (`audio.track_count_zero`); reachable via AudioPipeline::new |
| `Mixer::tracks` | 45-47 | REAL | — | trivial |
| `Mixer::set_volume` | 55-62 | REAL (UNREACHABLE in production) | — | `get_mut` bounds-checked → typed `audio.track_index_out_of_range`; no shipped caller (part1 finding) |
| `Mixer::volume` | 69-74 | REAL (UNREACHABLE in production) | — | same |
| `Mixer::mix` | 91-124 | REAL | F-002 | the LIVE core: geometry gates (track count, per-chunk plane count, per-plane length), zero-fill + ascending-order accumulate, zip-exact inner loop, `Ok(frames)`; no panic path found on any input shape (empty/zero-channel/zero-frame traced) |
| `tests::planes` (helper) | 132-134 | REAL | — | |
| `single_track_at_unity_is_bit_exact` | 136-144 | REAL | F-003 | PartialEq not bit-equality |
| `two_tracks_sum_in_track_order` | 146-154 | REAL | — | hand-verified exact |
| `per_track_volumes_apply_before_summing` | 156-168 | REAL | — | hand-verified (2.5) |
| `mute_leaves_the_other_track_untouched` | 170-179 | REAL | — | hand-verified (7) |
| `geometry_and_track_errors_are_typed` | 181-205 | REAL | F-002 | pins `chunk_length_mismatch` ×3 (the doc's `planar_plane_mismatch` never asserted — cannot fail for the doc's claim), track-range ×2, track_count_zero |
| `output_capacity_is_reused` | 207-216 | REAL | — | D-1 pin (capacity sum invariant across calls) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| single_track_at_unity / two_tracks / volumes / mute | YES | exact values, hand-reproduced; mutation `*dst = *dst * 0.5 + sample * gain` flips all four |
| geometry_and_track_errors_are_typed | YES (for the code's behavior) | exact code strings per branch — but it pins `chunk_length_mismatch` where the doc promises `planar_plane_mismatch` (F-002): the test and doc disagree, test wins |
| output_capacity_is_reused | YES | capacity-sum equality across two mixes; a per-mix reallocation (push-based or fresh Vec) flips it |
| Mutation that passes ALL tests: sign-bit/NaN-payload changes in the accumulate (F-003); also gain applied in descending track order (values here are order-insensitive: all inputs positive — a descending-order mutation passes the entire mix suite; the determinism ORDER claim is unpinned by values that would distinguish) | — | order-discrimination would need mixed-sign inputs across 3+ tracks |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`out[c][i] = Σ_t volume_t · in_t[c][i]`, accumulated in ascending track order … deterministic" | module docs 4-9 | REAL | code 111-122 exactly; green tests + AVI bench |
| "stage D's saturating clamp is where magnitude is bounded" | module docs 11-14 | FALSE (as shipped) | F-001: stage D is passthrough (convert.rs:256-278); clamp has no production caller |
| "doc 03 §4.8 b, insertion point C" (mixer = point C of the filter-chain sketch) | module docs 2 | REAL | pipeline stage order A→C→D matches doc 03 §4.8(b) sketch (03-engine-plan.md:584-591); point B (user filters) correctly absent in v0 |
| errors: `planar_plane_mismatch` (ragged) / `chunk_length_mismatch` (disagreement) | mix doc 85-90 | FALSE (first half) | F-002: ragged → chunk_length_mismatch in code |

## 7. Dead / duplicated / unreachable code
- In this file: none — every method is exercised (set_volume/volume by tests + API surface, production-dead per part1's finding).
- Cross-file dead code this audit exposes: `f32_to_s16_clamped` + `planes_to_s16_interleaved` (convert.rs:283-318) — tested, correct, ZERO production callers (F-001). Flagged for W07-A10 (convert.rs owner).
- `Mixer::set_volume`/`volume` — production-dead (no shipped composition adjusts per-track gain; the UI volume knob does not reach here — settings wave territory).

## 8. Dependencies & flags
- `media-types` (EngineError), `crate::error`, `crate::volume` — no external deps; no feature gates; no `unsafe` (verified by read).
- dev-dep `testkit::Must` for assertions.

## 9. Open questions for the next agent
- **W07-A10 (convert.rs)**: the dead clamp pair (F-001) — decide wire-or-delete; also the LE/NE seam (mix.rs audit confirms stage D writes `to_le_bytes` while the SampleFormat contract says native — see ms.rs audit §9, third handoff).
- **W07-A06 (volume.rs)**: confirm `Volume::linear()` rejects NaN/inf at construction (mix trusts it as a finite gain; a NaN gain would poison a whole plane — `single_track_at_unity` never tests a hostile gain).
- Volume-setting wave: `Mixer::set_volume` is API-without-a-caller — either the UI volume control should land here or the method should be documented as v1 surface.
