# AUDIT: crates/audio-engine/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-16 |
| Lines / functions | 128 / 0 code functions (crate root: 11 `pub mod`, 8 `pub use`, 1 doc-test) |
| File verdict | REAL (crate root; docs + re-exports) — with one FALSE doc claim and one unreachable exported subsystem |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:51 `use audio_engine::{AudioPipeline, PipelineSpec}`; AudioPipeline constructed at pipeline_host.rs:1600 and apps/web/src/host.rs:1746 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "EffectChain\|Eq3Band\|effects::" --glob '*.rs' apps crates` (exit 0 — ZERO consumers outside the effects module itself); `rg -n "use crate::effects" pipeline.rs mix.rs convert.rs buffer.rs resample.rs volume.rs` (exit 1 — not wired into the pipeline); `rg -n "src/lib.rs - " audit/runs/baseline/cargo-test-workspace.txt` (doc-test line 63 ok); `rg -n "unsafe\|std::thread" crates/audio-engine/src` (comment-only hits); sed quote verification |

## 1. One-paragraph verdict
The crate root is mostly an accurate map of a genuinely well-built crate: the four-stage pipeline description matches pipeline.rs, the doc example executes (baseline doc-test ok), the "Std-only; no unsafe" claim is true (rg: zero unsafe blocks, zero thread spawns, Cargo.toml deps = media-types + media-clock only), and every `pub use` names a real module. Two defects. First, the doc lies about scope: it says the filter-chain insertion point B "is empty in v0" while this very crate ships 807 lines filling it — `effects.rs` (EffectChain, Gain, Balance, Mute) plus `effects/eq.rs` (a 3-band RBJ biquad EQ) — re-exported three lines below the claim. Second, that entire effects subsystem is wired to nothing: no `use crate::effects` anywhere in the pipeline/mixer/convert path, and no consumer in any app or command — the `afeat-equalizer` matrix row's "partial" status cites this engine core as its evidence, but the core cannot process one sample in any shipped build.

## 2. What actually works (verified)
— verified by `rg -n "src/lib.rs - " audit/runs/baseline/cargo-test-workspace.txt` -> `test crates/audio-engine/src/lib.rs - (line 63) ... ok` (line 138 of the log): the doc example (pipeline construction, s16→f32 conversion, mix, 16-byte output) executes and asserts.
— verified by `rg -n "unsafe|thread" crates/audio-engine/src` -> only comment mentions (resample.rs:42, lib.rs:57); zero unsafe blocks, zero thread spawns — the "# Never" doc section holds.
— verified by reading Cargo.toml -> deps: media-types, media-clock (path), dev: testkit. Std-only, zero external crates: the header's claim is exact.

## 3. Findings
F-001 [S3]
Where: crates/audio-engine/src/lib.rs:17-19 vs lib.rs:110/123
Code (lib.rs:17-19, verbatim):
```
//!    active tracks with per-track gain. No reverb, no EQ, no
//!    crossfade — those are later filter-chain cards (doc 03 §4.8 b,
//!    insertion point B is empty in v0).
```
vs (lib.rs:110 and 123, verbatim):
```
pub mod effects;
...
pub use effects::{AudioEffect, Balance, EffectChain, Gain, MAX_EFFECTS, Mute};
```
What is wrong: the crate-root doc asserts the EQ/filter-chain insertion point is empty; the crate itself contains `effects.rs` (415 lines: EffectChain, Gain, Balance, Mute) and `effects/eq.rs` (392 lines: a three-band RBJ-cookbook biquad EQ with preamp) — 807 lines occupying exactly that insertion point, publicly exported two re-export groups below the claim. The doc is stale (written for card 2.C3, never updated for issue #33/D-012). A maintainer reading the crate root believes there is no EQ; a feature auditor reading eq.rs believes the EQ is staged. What correct looks like: stage list gains "Stage B′ — the (unwired) effect chain" and the "insertion point B is empty" sentence dies. Confidence: HIGH — Verification: read + wc/diff of module files.
F-002 [S3]
Where: crates/audio-engine/src/lib.rs:110,123 (the export surface), consumed by nothing
Code: (the `pub use` above)
What is wrong: `EffectChain`/`Gain`/`Balance`/`Mute`/`Eq3Band` have ZERO consumers outside the effects module's own tests: rg over every `.rs` in apps/ and crates/ finds no `audio_engine::EffectChain`, no `use crate::effects` in pipeline.rs/mix.rs/convert.rs/buffer.rs/resample.rs/volume.rs, no construction in any shipped host. `AudioPipeline` — the only composition in the tree — applies per-track volume through the Mixer (pipeline.rs:237-238) and never touches the effect chain. The subsystem is unreachable from every shipped entry point. Why it matters: the `afeat-equalizer` matrix row (docs/feature-matrix.toml:1671-1681) claims `status = partial` with evidence "crates/audio-engine/src/effects/eq.rs ... 86/0 crate tests" and notes "Partial: engine core is 3-band fixed-corner" — the notes honestly bound the *feature* (no 10-band, inert UI) but no line admits the engine core itself is outside the signal path: a "partial" that in the shipped product is 0%, not 50%. The same applies to the volume row's "engine Gain/Mute stages ... shipped" (feature-matrix.toml:1667-1668) — the live volume path is the Mixer's, not effects.rs's Gain/Mute. What correct looks like: wire EffectChain into AudioPipeline stage B (even an empty-by-default chain), or downgrade both matrix rows' evidence to "library code, unwired" until a composition constructs it. Confidence: HIGH — Verification: rg caller census (ran, zero hits). Matrix-row verdicts belong to the docs wave; logged here because the crate root is the export site.
F-003 [S4]
Where: crates/audio-engine/src/lib.rs:10-21 (the four-stage list)
Code:
```
//! 1. **Stage A — decoder-output normalization** ([`convert`]): any
```
What is wrong: the numbered stage description (A convert → B volume+channel map → C mixer → D convert) omits the effects stage entirely; combined with F-001 the map a reader builds of this crate is wrong in one specific place. Confidence: HIGH — Verification: read.

## 4. Per-function table
Not applicable — 0 functions; the auditable items are the module list and re-exports:
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| module decls (11) | 105-115 | REAL | every name matches a file/dir on disk (buffer, channel_map, convert, effects(+dir), error, governor, mix, pipeline, resample, volume) |
| re-exports (8) | 117-124 | REAL | all resolve; effects re-exports unwired (F-002) |
| doc example | 63-102 | REAL | doc-test ok in baseline (log line 138) |
| doc claims §What/§Never | 1-57 | REAL except F-001/F-003 | std-only, no-unsafe, no-threads verified |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lib.rs doc-test (line 63) | YES | constructs a pipeline, feeds 2 frames, asserts exact 16-byte output — breaks on conversion/mix regressions |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "insertion point B is empty in v0" | lib.rs:17-19 | FALSE | effects.rs + effects/eq.rs = 807 lines, exported at lib.rs:123 (F-001) |
| "No reverb, no EQ, no crossfade" | lib.rs:17 | FALSE (an EQ exists in-crate, unwired) | eq.rs:1-3 "Three-band biquad equalizer" |
| "Std-only; no `unsafe`" | lib.rs:57 | REAL | rg census: zero unsafe blocks/threads; Cargo.toml deps path-only |
| "no per-frame allocation after format negotiation" | lib.rs:25-26 | REAL-UNPROVEN here | asserted by tests/zero_alloc_resample.rs (resample path) + pipeline capacity tests; those are other files' rows |
| "sample counts ... map 1:1 to device samples" (conservation) | lib.rs:29-31 | REAL | pinned in pipeline/resample tests (other agents' rows); no contradiction found |

## 7. Dead / duplicated / unreachable code
The entire `effects` subsystem (807 lines) — exported public API, zero consumers, unreachable from every shipped entry point (F-002). Everything else in the crate is wired (pipeline ← CLI/web hosts; governor ← pipeline, armed only by tests — see governor.rs audit).

## 8. Dependencies & flags
media-types (path), media-clock (path); dev: testkit. No features, no unsafe, no threads. boundaries.toml row honored (per Cargo.toml comment).

## 9. Open questions for the next agent
- docs wave (feature-matrix rows afeat-equalizer, afeat-volume): reconcile "partial/engine core shipped" with the zero-consumer census here — the engine core is not in any signal path.
- Whoever lands `audio.set-eq` (D-012): the wiring seam is AudioPipeline stage B; until then eq.rs is library-only code.
