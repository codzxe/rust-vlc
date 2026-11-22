# AUDIT: crates/audio-engine/src/mix.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 217 / Mixer::{new, tracks, set_volume, volume, mix} + 6 tests (127-217, 41%) |
| File verdict | **REAL** — on the live per-quantum audio path, but only its degenerate single-track/unity case ever executes in the shipped product |
| Reachable from shipped entry point? | YES for `mix`: `lumen` → `AudioLimb` → `AudioPipeline::new(PipelineSpec { … tracks: 1 })` (apps/cli/src/pipeline_host.rs:1600-1603, the only production construction) → `pipeline.rs:343` `self.mixer.mix(&self.stage, &mut mixed)` per quantum. NO for the mixer's actual purpose: no shipped host ever constructs more than 1 track, and `set_volume` has zero production callers (only pipeline.rs's own tests and audio-output/tests/sine_sink_hash.rs). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (`PipelineSpec` constructions; `mixer.mix|.mix(` call sites; `set_volume|.volume(` callers tree-wide); sed verification. No cargo (static). |

## 1. One-paragraph verdict

A clean 90-line mixer: typed constructor (zero tracks refused), typed range errors on volume slot access, whole-chunk geometry validation (ragged planes, channel-count and frame-count disagreement all refused with `audio.chunk_length_mismatch`), fixed ascending-track accumulation for determinism (documented: float addition is not associative), output capacity reused across quanta (proven by test), and a bit-exact single-track-at-unity passthrough that the live pipeline relies on. The hostile question — "mix.rs: applied anywhere reachable?" — answers: the call is reachable (per quantum, on the CLI audio limb), but everything that distinguishes a *mixer* from a passthrough is not: the only production construction pins `tracks: 1` (pipeline_host.rs:1603) and no shipped code ever calls `set_volume`, so every mix in every shipped session is one unity-gain track copied into the output buffer. The summing, per-track gain and mute paths are test-only. This corroborates W03-A01's pipeline.rs finding ("multi-track mixer production-dead (hosts pin tracks:1)") and W03-A05's effects.rs finding (set_volume test-only).

## 2. What actually works (verified)

- `mix` (91-124): geometry validation (96-105) before any arithmetic; `out.resize_with(channels, …)` also *truncates* stale extra planes from a previous wider mix; per-plane clear+resize reuses capacity; ascending-track `*dst += *sample * gain` (119) is the documented deterministic order.
- `single_track_at_unity_is_bit_exact` (137-144): the property the live pipeline actually depends on — verified against the construction path (unity is the default at Mixer::new:40).
- Typed errors (35-37, 56-59, 69-73, 96-104): every failure named with a context code; test 182-205 pins all five.
- Capacity reuse (106-110): test 208-216 proves no reallocation on the second mix.

## 3. Findings

### F-001 [S3] — every shipped session mixes exactly one unity-gain track: the mixer is a passthrough in production
Where: apps/cli/src/pipeline_host.rs:1600-1603 (the pin), mix.rs:4-5 (the contract that never runs)
Code (apps/cli/src/pipeline_host.rs:1600-1603):
```rust
    let pipeline = AudioPipeline::new(PipelineSpec {
        …
        tracks: 1,
```
Code (mix.rs:4-5):
```rust
//! "Straight" is the card's word and the contract: `out[c][i] =
//! Σ_t volume_t · in_t[c][i]`, accumulated in ascending track order —
```
What is wrong: the only production `PipelineSpec` pins one track; `Mixer::set_volume` has no production caller (rg: pipeline.rs:237-238 forwards it, but its callers are pipeline.rs's own tests at 533/686 and audio-output/tests/sine_sink_hash.rs:70); therefore `Σ` is always a one-term sum of `sample × 1.0` and `volume_t` is always UNITY in the shipped product. The determinism property the module advertises (byte-stable multi-track sums for the sink-buffer hash) is only ever exercised with one term.
Why it matters: card 2.C3's deliverable — a multi-track mixer — exists as library code and nothing else. No user of any shipped binary can reach a real mix (there is no multi-track input path and no volume control; see also the effects.rs audit: the `audio.set-volume` command has no handler).
What correct looks like: either a host path that wires N audio limbs (secondary audio, doc 05's `[PROPOSAL]`) or the honest label "v0: passthrough mixer".
Confidence: HIGH — Verification: read (rg trace; cross-agent consensus with W03-A01 pipeline.rs and W03-A05 effects.rs audits).
Severity: S3 — the code is correct and the single-track case is itself load-bearing; the defect is the unreachable feature, not a wrong behavior.

### F-002 [S4] — `set_volume`'s doc parenthetical is vacuous
Where: crates/audio-engine/src/mix.rs:52-54
Code:
```rust
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_index_out_of_range` (the gain's own validation
    /// errors propagate from [`Volume`]).
```
What is wrong: the parameter is an already-constructed, already-validated `Volume` — no gain-validation error can "propagate" from this call; the only error is the index range named above the parenthetical.
Why it matters: misleading doc on a public API; a reader may expect `set_volume` to validate and would double-validate.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Mixer::new | 35-42 | REAL | — | zero tracks → typed `audio.track_count_zero`; unity defaults |
| Mixer::tracks | 45-47 | REAL | — | accessor, unused in production (spec-driven) |
| Mixer::set_volume | 55-62 | REAL (unreachable) | S3 | typed range error; no production caller (F-001) |
| Mixer::volume | 69-74 | REAL (unreachable) | — | typed range error; test-only callers |
| Mixer::mix | 91-124 | REAL | — | live per quantum; geometry validation; deterministic order; capacity reuse; degenerate 0-channel input passes as Ok(0) (all-empty chunks) — typed-safe, unreachable geometry in practice |

## 5. Test honesty (6 tests, lines 127-217)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| single_track_at_unity_is_bit_exact (137) | YES | exact equality on two planes — the production case |
| two_tracks_sum_in_track_order (147) | YES | exact sums — but tests the never-shipped 2-track case |
| per_track_volumes_apply_before_summing (157) | YES | 0.5×+2.0× exact; accessors verified |
| mute_leaves_the_other_track_untouched (171) | YES | MUTE zeroing |
| geometry_and_track_errors_are_typed (182) | YES | five error cases with exact context codes |
| output_capacity_is_reused (208) | YES | capacity-sum equality across two mixes |

All real; UNVERIFIED by run here. Tests cover the feature surface honestly, including the parts production never reaches.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| `out[c][i] = Σ_t volume_t · in_t[c][i]` ascending order | mix.rs:4-5 | REAL (code) / UNREACHABLE (multi-track form in product) | F-001 |
| "deterministic (byte-stable across runs and hosts, which the sink-buffer hash AC depends on)" | mix.rs:7-9 | REAL | order fixed at 111-121; sine_sink_hash.rs exercises it (test-side) |
| "Sum overflow stays in the f32 domain … stage D's saturating clamp is where magnitude is bounded" | mix.rs:11-14 | REAL-UNPROVEN | the clamp lives in the interleave/pooled-output stage (pipeline.rs, W03-A01's file) — not re-verified here |
| capacity reused across calls | mix.rs:77-78 | REAL | test 208 |

## 7. Dead / duplicated / unreachable code

- `set_volume`/`volume`: production-unreachable (F-001). Not duplicated.

## 8. Dependencies & flags

- Depends on `crate::volume::Volume` (next file in this agent's list) and `crate::error` helpers. No features, no unsafe, allocation only in `out`'s growth (reused thereafter).

## 9. Open questions for the next agent

- volume.rs (mine, next): confirm `Volume::linear()` and UNITY/MUTE semantics that this file leans on.
- Wave 18: `afeat-*` mixer rows — the "mixer" claim should be graded on the passthrough reality.
- apps/cli pipeline_host owner: why `tracks: 1` is hard-pinned (single-audio-limb v0) — the one-line answer belongs in the remediation map.
