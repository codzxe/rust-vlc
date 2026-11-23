# AUDIT: crates/audio-engine/src/mix.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-30 |
| Lines / functions | 217 / 6 (+7 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CLI: apps/cli/src/host.rs -> pipeline_host.rs:51,1600 `AudioPipeline::new(PipelineSpec {...})` -> audio-engine/src/pipeline.rs:122 `mixer: Mixer::new(spec.tracks)?` -> pipeline.rs:343 `self.mixer.mix(&self.stage, &mut mixed)`; web host same via apps/web/src/host.rs:74. Feature-free crate (no gates). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/audio_engine-78c3118f7cd3bd18 mix` -> `test result: ok. 7 passed; 0 failed` (exit 0); rg traces of Mixer/clamp callers (exit 0); sed quote verification (exit 0) |

NOTE: assignment row 835 says `crates/audio_engine/src/mix.rs`; the real path is `crates/audio-engine/src/mix.rs` (hyphenated crate dir). Audited the real file.

## 1. One-paragraph verdict
The mixing math is real and correct: `out[c][i] = Σ_t gain_t · in_t[c][i]` in fixed ascending track order (the determinism claim holds — no FP contraction in Rust), per-track volume applied pre-sum, whole-chunk geometry validation, typed errors, zero per-call allocation (capacity reuse test-pinned). Hostile-focus answers: (1) real mixing math — YES, verified by execution; (2) clipping — NONE in the mixer by design, and the module doc's claim that "stage D's saturating clamp" bounds magnitude downstream is FALSE: stage D emits raw f32 bytes (`planes_to_f32_interleaved_into`, pipeline.rs:412) with no clamp, and the only saturating clamp in the crate (`f32_to_s16_clamped`/`planes_to_s16_interleaved`, convert.rs:283,306) has zero production callers — dead code; (3) reachable from playback — YES through AudioPipeline in both shipped hosts, though every sink today is the NullSink (cpal not admitted), so the unclamped >±1.0 mix output is currently swallowed by a null device, not heard.

## 2. What actually works (verified)
- `./target/debug/deps/audio_engine-78c3118f7cd3bd18 mix` -> `test result: ok. 7 passed; 0 failed` — includes `single_track_at_unity_is_bit_exact` (exact byte equality through the mixer), `two_tracks_sum_in_track_order` (1+10=11, 2+20=22...), `per_track_volumes_apply_before_summing` (0.5·1 + 2·1 = 2.5), `mute_leaves_the_other_track_untouched`, `output_capacity_is_reused` (capacity sums equal across calls).
- Reachability chain read end-to-end (see header): CLI play path constructs the Mixer per session.
- `Volume::from_linear` validation exists and `set_volume` propagates it (volume.rs:44,151 test `linear_construction_rejects_bad_values`).

## 3. Findings
F-001 [S3]
Where: crates/audio-engine/src/mix.rs:11-14 (claim) vs crates/audio-engine/src/pipeline.rs:412+427-430 and crates/audio-engine/src/convert.rs:283,306
Code (mix.rs:11-14):
```
//! Sum overflow stays in the f32 domain (values may exceed ±1.0 or
//! grow toward infinity — *data* at this stage); stage D's saturating
//! clamp is where magnitude is bounded (doc 03 §4.8 e's soft limiter
//! is the later hardening).
```
and convert.rs:283 `pub fn f32_to_s16_clamped(x: f32) -> i16 {` — rg over the whole repo shows its only callers are convert.rs's own tests; the live stage D (`pipeline.rs` pooled_slot -> `planes_to_f32_interleaved_into`, convert.rs:256) writes `plane[f].to_le_bytes()` raw, no clamp.
What is wrong: the doc asserts magnitude is bounded downstream; it is not, anywhere on a shipped path. Two unity full-scale tracks sum to ±2.0 and flow to the sink unclamped. Why it matters: the mixer's own no-clip design is defensible, but the safety claim is false, and the one clamp that exists is dead code — the first real audio backend will inherit unbounded f32 output. Blast radius today: none audible (every backend is Null/Unsupported — cpal not admitted, audio-output/Cargo.toml records the refusal). What correct looks like: clamp (or soft-limit) in stage D, or delete the claim and the dead s16 clamp. Confidence: HIGH — Verification: read + rg (no caller found) + tests run.
F-002 [S4]
Where: crates/audio-engine/src/mix.rs:86-88
Code:
```
    /// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
    /// `audio.planar_plane_mismatch` (a track's planes are ragged) or
    /// `audio.chunk_length_mismatch` (tracks disagree on plane count
```
What is wrong: the mixer never emits `audio.planar_plane_mismatch` — ragged planes (line 102) return `error::chunk_alignment()` like every other geometry violation, and the file's own test asserts ragged planes -> `audio.chunk_length_mismatch` (tests.rs:190-192). The `planar_plane_mismatch` code exists in the crate (channel_map.rs:147, convert.rs:46) but not on this path. Why it matters: the documented error taxonomy for this function is wrong; a caller matching on the documented code dead-ends. What correct looks like: fix the doc or emit the distinct code. Confidence: HIGH — Verification: read + tests run.
F-003 [S4]
Where: crates/audio-engine/src/mix.rs:99-100
Code:
```
        let channels = inputs.first().map_or(0, Vec::len);
        let frames = inputs.first().map_or(0, |c| c.first().map_or(0, Vec::len));
```
What is wrong: a zero-plane (or zero-frame) input set mixes "successfully" to an empty output — `Ok(0)`/empty `out` with no typed diagnostic, while zero *tracks* is a hard `audio.track_count_zero` error. Inconsistent degeneracy policy (silent empty success vs typed refusal). Why it matters: a wiring bug upstream (no planes staged) produces silent silence instead of a typed error. What correct looks like: reject channels == 0 as `audio.chunk_length_mismatch` (or a dedicated code). Confidence: HIGH — Verification: read (mutation reasoning; no such caller exists in-tree — the pipeline always stages ≥1 plane).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Mixer::new | 35-42 | REAL | — | zero-track typed error |
| Mixer::tracks | 45-47 | REAL | — | trivial |
| Mixer::set_volume | 55-62 | REAL | — | bounds-checked; Volume validation propagates |
| Mixer::volume | 69-74 | REAL | — | bounds-checked |
| Mixer::mix | 91-124 | REAL | — | correct sum math, fixed order, geometry-validated; F-002 doc, F-003 degeneracy |
| 7 tests | 136-216 | REAL tests | — | mutation-sensitive (exact values, typed codes, capacity) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| single_track_at_unity_is_bit_exact | YES | `assert_eq!(out, input)` full-plane equality |
| two_tracks_sum_in_track_order | YES | exact sums 11/22/44/88 |
| per_track_volumes_apply_before_summing | YES | 0.5+2.0 gains exact 2.5; also volume() round-trip |
| mute_leaves_the_other_track_untouched | YES | exact 7.0 |
| geometry_and_track_errors_are_typed | YES | five distinct codes asserted |
| output_capacity_is_reused | YES | capacity sums compared across calls |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| `out[c][i] = Σ_t volume_t · in_t[c][i]`, ascending track order, deterministic | mix.rs:4-9 | REAL | code:111-122 matches; tests exact |
| "no reverb, no EQ, no crossfade, no dither" | mix.rs:6-7 | REAL | the only per-sample op is `*dst += *sample * gain` |
| "stage D's saturating clamp is where magnitude is bounded" | mix.rs:12-13 | FALSE | F-001: no clamp on any live path; s16 clamp is dead code |
| error codes `audio.planar_plane_mismatch` / `audio.chunk_length_mismatch` | mix.rs:86-88 | FALSE (first code) | F-002: both cases emit chunk_length_mismatch |
| "capacity is reused across calls (doc 03 §4.8 c)" | mix.rs:77-78 | REAL | output_capacity_is_reused test |

## 7. Dead / duplicated / unreachable code
- convert.rs:283 `f32_to_s16_clamped` and :306 `planes_to_s16_interleaved`: zero production callers (rg over crates+apps; only convert.rs tests) — the "saturating clamp" the module doc leans on (F-001).
- (This file itself: none.)

## 8. Dependencies & flags
No external deps; not feature-gated. Consumers: audio-engine/pipeline.rs (live), exported `pub use mix::Mixer` (lib.rs). Volume from crate::volume (validated construct). testkit dev-dep only.

## 9. Open questions for the next agent
- pipeline.rs (staging, drift seam, pooled slots) and convert.rs are separate assignments — the dead s16 clamp and the raw-f32 stage D live there; F-001's severity may rise to S2 the moment a real device backend (platform-*) lands. Check whether any platform-* crate already consumes AudioPipeline f32 output.
- Does doc 03 §4.8 e promise the soft limiter as shipped or future? (Doc audit territory.)
