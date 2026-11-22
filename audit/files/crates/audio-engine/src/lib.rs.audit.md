# AUDIT: crates/audio-engine/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 128 / 0 functions — 10 module declarations, ~20 re-exports, 1 doctest (63-108), module docs (1-58) |
| File verdict | **REAL** — an honest crate manifest; its doctest is a genuine end-to-end integration test |
| Reachable from shipped entry point? | YES (crate level): `audio-engine` is a dependency of apps/cli (Cargo.toml:48) and apps/web (Cargo.toml:67); the AudioLimb constructs `AudioPipeline` on the live path (pipeline_host.rs:1600). The doctest itself runs only under `cargo test`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (`unwrap\(|expect\(|panic!|unsafe` over crates/audio-engine/src — zero real hits, only doc-comment mentions at lib.rs:57 and resample.rs:42); cat crates/audio-engine/Cargo.toml (dependency audit). No cargo (static). |

## 1. One-paragraph verdict

A manifest file that does what a manifest should: declares the ten modules, re-exports the public surface, and carries one doctest that is a real integration test — build a pipeline, push an s16 stereo chunk through `process` → `mix` → `take_output`, assert the 16-byte interleaved-f32 output (lines 100-105); it would fail if stage A conversion, the unity passthrough, or stage D interleave broke. The doc's honesty claims check out under grep: "No panics, no unwraps" — zero `unwrap(`/`expect(`/`panic!` in the crate's src (only comment mentions); "Std-only; no `unsafe`" — zero `unsafe` blocks, and the Cargo.toml carries only workspace-internal deps (media-types, media-clock, dev testkit). The one defect is cosmetic-but-symptomatic: the re-export surface (line 123-124: `EffectChain, Gain, Balance, Mute, DriftGovernor`) advertises subsystems that no shipped code ever constructs (the effects.rs and governor.rs audits), so the crate's public API reads as an aspirational catalog while the module doc correctly says "insertion point B is empty in v0".

## 2. What actually works (verified)

- Doctest (63-108): end-to-end unity-passthrough with an exact output-size assertion (`Some(16)` = 2 frames × 2 channels × 4 bytes); exercises `AudioPipeline::new`, `set_volume`, `process`, `mix`, `take_output`. Runs under `cargo test` (W00 baseline: workspace suite 1751 pass / 2 fail, both failures in first-vertical — consistent with this doctest passing; UNVERIFIED individually).
- "No panics, no unwraps" (line 51): rg over crates/audio-engine/src for `unwrap\(|expect\(|panic!|todo!|unimplemented!` → zero hits.
- "Std-only; no `unsafe`" (line 57): rg `unsafe` → zero blocks (two doc-comment mentions only); Cargo.toml dependencies are `media-types` + `media-clock` (path, workspace-internal) + dev-only `testkit` — no external crates.
- Module list (110-119) matches the directory: buffer, channel_map, convert, effects, error, governor, mix, pipeline, resample, volume — all exist (verified by this agent's audits of mix/volume/governor and W03-A01/A05's of pipeline/effects/resample/channel_map).
- The sample-count-conservation doc (29-45) accurately describes the design: rate-preserving stages, the micro-resampler as the one sanctioned exception, track-rate mismatch as a typed error.

## 3. Findings

### F-001 [S4] — the re-export surface advertises subsystems no shipped code constructs
Where: crates/audio-engine/src/lib.rs:123-124 vs the module doc at 16-19
Code (lib.rs:123-124):
```rust
pub use effects::{AudioEffect, Balance, EffectChain, Gain, MAX_EFFECTS, Mute};
pub use governor::{DriftGovernor, DriftStats};
```
Code (lib.rs:16-19, the honest half):
```rust
//! 3. **Stage C — the mixer** ([`Mixer`]): a *straight* sum of all
//!    active tracks with per-track gain. No reverb, no EQ, no
//!    crossfade — those are later filter-chain cards (doc 03 §4.8 b,
//!    insertion point B is empty in v0).
```
What is wrong: no factual error — the pipeline's insertion point B *is* empty — but the crate's public API surface (`EffectChain`, `Gain`, `Balance`, `Mute`, `DriftGovernor`) presents those subsystems as available when, per the effects.rs audit (W03-A05: nothing constructs an EffectChain in any shipped binary; the `audio.set-volume` command has no handler) and this agent's governor.rs audit (F-001: no production arming), no shipped entry point can reach any of them. A consumer reading lib.rs's exports would conclude the audio path has effects and drift governance; it has dead exports.
Why it matters: API-surface-as-marketing — the same pattern the feature matrix exhibits, one level down. Blast radius: confusion only.
What correct looks like: either `#[doc(hidden)]`-style demotion or a doc paragraph stating which exports are dormant.
Confidence: HIGH — Verification: read (cross-referenced with the effects.rs and governor.rs audits' caller traces).

## 4. Per-function table

Not applicable (no functions). Items audited instead:

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| module docs (stages A-D, conservation, errors, Never) | 1-58 | REAL | claims verified by grep (§2) |
| doctest | 63-108 | REAL | genuine integration test with exact assertion |
| `pub mod` × 10 | 110-119 | REAL | all modules exist and are audited |
| re-exports | 121-128 | REAL | resolve; effects/governor exports dormant (F-001) |

## 5. Test honesty

The doctest asserts a real output (`buffer.map(|b| b.data.len()), Some(16)`) after a real conversion chain — it would fail under mutations of stage A/D. Not a tautology. UNVERIFIED by execution (no cargo); W00 baseline consistency noted in §2.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No panics, no unwraps; malformed payloads are typed Corrupt errors" | lib.rs:51-52 | REAL | rg: zero unwrap/expect/panic in src |
| "Std-only; no `unsafe`" | lib.rs:57 | REAL | rg: zero unsafe; Cargo.toml internal-only deps |
| "insertion point B is empty in v0" | lib.rs:19 | REAL | pipeline has no effect-chain call (effects.rs audit) |
| "The pipeline is rate-preserving by construction … ZERO restores the 1:1 path bit-exactly" | lib.rs:31-41 | REAL-UNPROVEN | resample.rs is W03-A01's file; the ZERO path is the one production actually runs |
| "governed by [`DriftGovernor`]" | lib.rs:37 | FALSE (as a product statement) | the governor is never armed in any shipped build (governor.rs F-001) — the type graph governs, the runtime never does |
| "zero external dependencies" (Cargo.toml comment) | Cargo.toml | REAL | only path deps on workspace crates |

## 7. Dead / duplicated / unreachable code

- Dormant re-exports: `EffectChain/Gain/Balance/Mute/MAX_EFFECTS` (effects.rs audit), `DriftGovernor` (governor.rs audit), `MicroResampler` (constructed but only ever at ZERO). See F-001.

## 8. Dependencies & flags

- Workspace-internal only: media-types, media-clock (production), testkit (dev). `[lints] workspace = true`. No features, no unsafe. Clean.

## 9. Open questions for the next agent

- None for this file. The crate-level summary for wave docs: of the ten modules, the *live* production surface is convert + channel_map(identity) + mix(1-track unity) + buffer + pipeline + resample(ZERO bypass) — everything else (effects, governor, volume constructors, multi-track) is dormant.
