# AUDIT: crates/audio-engine/src/effects.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-21 |
| Lines / functions | 415 / 24 items |
| File verdict | UNREACHABLE (real, tested code; zero production callers from any shipped entry point) |
| Reachable from shipped entry point? | NO. Chain of evidence: `lumen` CLI imports only `audio_engine::{AudioPipeline, PipelineSpec}` (apps/cli/src/pipeline_host.rs:51) + `AudioSink` (pipeline_host.rs:619) + `DriftStats` (pipeline_host.rs:671,1068); `AudioPipeline` (pipeline.rs) never references `EffectChain`/`AudioEffect`/`Gain`/`Mute`/`Balance`/`Eq3Band` (rg over crates/audio-engine/src: only lib.rs:123 re-export, effects/eq.rs internal use, and tests). apps/web/src/host.rs:74 likewise imports only `AudioPipeline, PipelineSpec`. apps/desktop-tauri has no audio dependency at all (rg "audio" apps/desktop-tauri/Cargo.toml: none). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg "EffectChain\|AudioEffect\|Eq3Band\|BiquadFilter" (all callers — none outside the crate); rg "audio_engine::" apps/cli apps/desktop crates/playback-engine (imports enumerated above); rg "audio_engine\|EffectChain" apps/web/src apps/desktop-tauri/src (host.rs:74 only; tauri none); rg "set-volume\|set_volume" repo (real path: playback-engine command → pipeline.rs:237-238 `set_volume` → `Mixer::set_volume`); rg -n "effects" audit/runs/baseline/test-workspace.log.txt:63-82 (21 effects/eq unit tests ok) + awk unit-test summary ("86 passed; 0 failed" for audio-engine); rg "pub fn from_linear\|from_db" volume.rs:44-71 (NaN/inf discipline check); sed -n quote verification (36-39, 140-143, 178-181). |

## 1. One-paragraph verdict
The DSP in this file is small, correct, and honestly tested: the constant-power pan law is mathematically right and pinned by tests to 1e-7, the chain is bounded, fail-fast, allocation-free in `process`, and every stage validates its shape or zips instead of indexing. But none of it is wired to anything: no shipped entry point constructs an `EffectChain`, `Gain`, `Mute`, `Balance`, or `Eq3Band` — the CLI, web, and tauri builds use the audio pipeline/mixer path only, and even `AudioPipeline::set_volume` → mixer has no production caller (per audit/files/crates/audio-engine/src/pipeline.rs#part1.audit.md). The file's own doc comments nevertheless present `Gain` as "the effect-chain's front door for `audio.set-volume`", and docs/feature-matrix.toml:1667 cites this file as the evidence that the UI volume slider and mute switch are "live" — a false attribution: the live path (where it exists) is UI → command → mixer, not this file. Two smaller doc defects: the `Mute` zero-sign claim is wrong for negative samples, and the module's "never NaN gains" promise is breakable via `Volume::from_db` overflow (cross-file, flagged to W07-A06).

## 2. What actually works (verified)
— verified by audit/runs/baseline/test-workspace.log.txt:63-82 → all 11 `effects::tests` + 10 `effects::eq::tests` ok (gain amplitude/identity, mute, pan-law center/hard-stops/per-channel, stereo rejection, chain order/bound/identity, gain∘mute); audio-engine unit suite 86 passed / 0 failed (log summary line).
— verified by read: pan law `L=cos((p+1)π/4), R=sin((p+1)π/4)` (238-241) — at p=0 both √2/2, L²+R²=1 at every p (cos²+sin² of the same angle); `Balance::process` (245-263) checks `planes.len()==2` before touching data and `split_at_mut(1)`+`zip` cannot panic or overlap; `EffectChain::process` (132-137) is fail-fast in push order; `push` enforces MAX_EFFECTS=8 before allocation (104-110); no allocation, no unwrap, no panic path in any `process` (162-171, 186-193, 245-263).

## 3. Findings

### F-001 [S3] — the entire effects module is unreachable from every shipped entry point
Where: crates/audio-engine/src/effects.rs:60-268 (trait + all stages)
Code:
```rust
pub trait AudioEffect {
    fn process(&mut self, planes: &mut [Vec<f32>]) -> Result<(), EngineError>;
```
What is wrong: No production code path constructs any of this. The shipped CLI's only audio_engine imports are `AudioPipeline`/`PipelineSpec`/`AudioSink`/`DriftStats` (apps/cli/src/pipeline_host.rs:51,619,671,1068); `pipeline.rs` never mentions effects; apps/web/src/host.rs:74 imports only `AudioPipeline, PipelineSpec`; desktop-tauri has no audio dep. The only constructors of `EffectChain` are lib.rs's doc example, eq.rs's test, and this file's tests.
Why it matters: A whole DSP surface (chain + 4 stage types + EQ) ships in every binary and is advertised in the feature matrix, yet a user cannot reach a single stage: volume/mute commands route through the mixer (pipeline.rs:237-238), and there is no balance/EQ command executor at all (feature-matrix.toml:2782 admits balance/EQ sliders are "inert-with-reason pending audio.set-balance/audio.set-eq"). Dead weight in the shipped build and an inflated capability story.
What correct looks like: Either wire `EffectChain` into `AudioPipeline` (the documented "insertion point B") or mark the module experimental/not-yet-wired in the feature matrix and lib.rs docs.
Confidence: HIGH — Verification: read (rg call-site enumeration above); not executed (no shipped path to execute).

### F-002 [S3] — `Gain`'s doc claims to be the engine-side `audio.set-volume` front door; the real path is the mixer
Where: crates/audio-engine/src/effects.rs:140-143
Code:
```rust
/// A gain stage: one [`Volume`] scalar multiply on every sample of
/// every channel (the effect-chain's front door for `audio.set-volume`
/// — engine-side it is the same multiply the mixer applies, exposed as
```
What is wrong: `audio.set-volume` resolves to `AudioPipeline::set_volume` → `self.mixer.set_volume(track, volume)` (crates/audio-engine/src/pipeline.rs:237-238). No command or pipeline path instantiates `Gain`. The parenthetical presents this stage as the command's engine-side home.
Why it matters: The comment is the file's justification for existing ("the front door for audio.set-volume"); it misdescribes the shipped wiring. docs/feature-matrix.toml:1667 compounds it by citing effects.rs as evidence that the "ui settings area volume slider + mute switch [are] live via audio.set-volume/audio.set-mute" — the liveness, where real, is mixer code, not this file (and per the pipeline audit, even `set_volume` has no production caller).
What correct looks like: "A gain stage over the chain's own format — the mixer remains the engine-side volume authority (`AudioPipeline::set_volume`); this stage exists so filter cards compose with gain deterministically once the chain is wired."
Confidence: HIGH — Verification: read (pipeline.rs:237-238 + rg call sites + feature-matrix row 1667).

### F-003 [S4] — `Mute` doc claims "every finite sample becomes +0.0"; negative samples become −0.0
Where: crates/audio-engine/src/effects.rs:178-181
Code:
```rust
/// A mute stage: multiply by exactly zero (the [`Volume::MUTE`]
/// semantics — a NaN sample's payload sign is preserved, every finite
/// sample becomes +0.0; un-muting is removing the stage, not a state
```
What is wrong: IEEE 754 sign rule: `(−x) · (+0.0) = −0.0`. A negative finite sample (e.g. −5.0) becomes −0.0 (bits 0x80000000), not +0.0. The test `*s == 0.0` passes for −0.0 as well, so the pinned behavior and the prose disagree at the bit level.
Why it matters: The doc sells bit-level semantics ("exactly zero", "+0.0") it does not deliver; any downstream bit-exactness check (dither detection, bit-compare against reference players) sees −0.0 planes.
What correct looks like: "every finite sample becomes a zero (−0.0 for negative inputs, per IEEE sign rules)" — or `*sample = 0.0` (assignment, not multiply) if +0.0 is the contract.
Confidence: HIGH — Verification: read (IEEE 754 multiply sign rule); not executed (no runtime here — REQUEST-ORCH-RUN not warranted for an IEEE identity).

### F-004 [S4] — module's "never NaN gains" promise is breakable through `Volume::from_db`
Where: crates/audio-engine/src/effects.rs:36-39
Code:
```rust
//! # Never
//!
//! Allocation in `process`, panics, unwraps, NaN gains (rejected at
//! construction, the [`Volume`] discipline).
```
What is wrong: The referenced discipline rejects non-finite *inputs*, not non-finite *results*: `Volume::from_db(db)` (volume.rs:62-71) accepts any finite db and computes `10^(db/20)` — `from_db(800.0)` yields `linear = +inf` in f32 (10^40 > f32::MAX) and returns `Ok`. A `Gain` built from that volume multiplies every sample into ±inf, and inf·0.0 in a following `Mute` produces NaN — the exact outcome the module promises can never happen.
Why it matters: The promise is the module's safety story; it holds only for `from_linear`, not the other sanctioned constructor. (Root defect is in volume.rs — W07-A06's file; recorded here because the promise is made here.)
What correct looks like: `from_db` validates `linear.is_finite()` after the powf (typed `audio.gain_not_finite`), or the module doc scopes the promise to from_linear.
Confidence: HIGH — Verification: read (volume.rs:44-71); f32 overflow of 10^40 to +inf is IEEE-defined.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-39 | REAL (code) / doc defects | S4 | pan-law section correct; "Never" promise breakable (F-004); "insertion point B … empty in card 2.C3" admits the chain is not wired — consistent with F-001 |
| `pub mod eq` + re-exports | 46-48 | REAL | — | eq.rs is W07-A06's file |
| `MAX_EFFECTS` | 50-52 | REAL | — | 8, enforced at push |
| `trait AudioEffect::process` | 68 | REAL | — | in-place, no alloc by contract |
| `trait AudioEffect::name` | 72 | REAL | — | &'static str |
| `EffectChain`/Default | 81-89 | REAL | — | Vec<Box<dyn>>, capacity 8 |
| `EffectChain::new` | 93-97 | REAL | — | trivial |
| `EffectChain::push` | 104-110 | REAL | — | bound-checked before push; typed `audio.effects_chain_full` |
| `EffectChain::len`/`is_empty`/`clear` | 113-125 | REAL | — | trivial |
| `EffectChain::process` | 132-137 | REAL | — | fail-fast, in order; empty = no-op (no copy) |
| `Gain::new`/`volume` | 152-159 | REAL | — | delegates discipline to Volume (see F-004) |
| `Gain::process` | 163-171 | REAL | — | one multiply per sample; no alloc; inf volume → inf samples (F-004) |
| `Gain::name` | 173-175 | REAL | — | "gain" |
| `Mute::process` | 186-193 | REAL | — | *= 0.0; −0.0 for negatives (F-003) |
| `Mute::name` | 195-197 | REAL | — | "mute" |
| `Balance::{UNITY,LEFT,RIGHT}_GAIN` | 208-215 | REAL | — | consts match law; trig-rounding caveats documented and test-pinned |
| `Balance::new` | 224-229 | REAL | — | rejects non-finite / out-of-[−1,1]; no clamping |
| `Balance::position` | 232-234 | REAL | — | trivial |
| `Balance::gains` | 238-241 | REAL | — | cos/sin of (p+1)·π/4 — verified law |
| `Balance::process` | 245-263 | REAL | — | stereo check → typed error; split_at_mut(1); zip (no OOB even if plane lengths diverge); no alloc |
| `Balance::name` | 265-267 | REAL | — | "balance" |
| tests `sine`/`peak` helpers | 277-285 | REAL | — | integer-period sine; fold max-abs |
| 10 tests | 287-414 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| gain_scales_sine_amplitude | YES | known-signal in → expected amplitude out (tolerance 1e-3) |
| gain_identity_is_bit_exact | YES | assert_eq on the plane — a non-1.0 multiply flips bits |
| mute_zeroes_everything | YES (mostly) | all==0.0; note: cannot distinguish +0.0 from −0.0 (F-003) — a `*sample = 0.0` vs `*= 0.0` change is invisible to it |
| balance_center_is_minus_3db_constant_power | YES | pins UNITY_GAIN and L²+R²=1 at 0.37 |
| balance_hard_stops_are_pinned | YES | 1e-7 tolerances on the trig extremes |
| balance_scales_each_channel_by_its_law_gain | YES | hard-left: L peak 0.8 kept, R < 1e-7 |
| balance_rejects_out_of_range_and_non_stereo | YES | 1.01 / NaN / mono all must error |
| chain_runs_stages_in_push_order | YES | pins composition value 0.2 (gain 0.5 → hard-right) |
| empty_chain_is_identity_noop | YES | bit-equal after process |
| chain_bound_is_a_wall | YES | 9th push must error; clear works |
| gain_mute_compose_to_silence | YES | all zeros after 2.0× → mute |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| "EffectChain v0 + Gain/Mute AudioEffect stages, commit 8bd13ed; ui settings area volume slider + mute switch live via audio.set-volume/audio.set-mute" | docs/feature-matrix.toml:1667 | FALSE (attribution) | the live path is mixer: pipeline.rs:237-238; this file has zero production callers (F-001/F-002); the "live" claim itself is doubtful — pipeline audit found no production caller for set_volume either |
| "effects tests 86/0 in crate" | docs/feature-matrix.toml:1667 | REAL (number) / inflated (label) | 86/0 is the audio-engine crate unit total (baseline summary); effects.rs+eq.rs contribute 21 of them, not 86 |
| "Gain … front door for audio.set-volume" | effects.rs:141-143 | FALSE | pipeline.rs:237-238 (F-002) |
| pan law constants and −3.01 dB center | effects.rs:19-34, 207-215 | REAL | gains() math verified; tests pinned (baseline :72-75) |
| "no silent mono fold" for non-stereo Balance | effects.rs:32-34 | REAL | typed error at 246-248; test at 352-363 |
| "Allocation in process, panics, unwraps, NaN gains: never" | effects.rs:36-39 | REAL for alloc/panic/unwrap (read); FALSE for NaN via from_db overflow | F-004 |
| "at most MAX_EFFECTS stages … bounded memory by construction" | effects.rs:14-16 | REAL | push bound at 104-110; test chain_bound_is_a_wall |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (F-001): trait, chain, all four stage types. Consumers: lib.rs re-export (lib.rs:123), eq.rs (fellow unreachable module), doc examples, tests. No duplication found within the file.

## 8. Dependencies & flags
- Uses `media_types::EngineError`, `crate::error`, `crate::volume` — all internal. No external deps, no features, no unsafe. The `eq` submodule is compiled unconditionally (pub mod), so the dead surface also costs compile time in every shipped build.

## 9. Open questions for the next agent
- W07-A06 (volume.rs): fix or pin `Volume::from_db` result-finiteness (F-004 root) — `from_db(800.0)` currently returns an infinite gain.
- W07-A06 (eq.rs): same reachability question — confirm no production caller (my rg found none) and whether the EQ state (biquad memories) is reset anywhere on seek (stateful stage in a chain nothing drives).
- Engine owners: if/when the chain is wired at "insertion point B", the Mute −0.0 (F-003) and Balance unequal-plane zip-silence choices become audible-path semantics — decide them consciously.
- Hostile-question note for the wave summary: effects.rs contains NO sample-rate conversion; the resampler is resample.rs (W07-A02) and the mixer mix.rs (W07-A02).
