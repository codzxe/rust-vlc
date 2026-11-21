# AUDIT: crates/audio-engine/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Lines / functions | 128 / 20 items (module docs 1-58; doctest 59-108; 10 `pub mod` declarations; 8 `pub use` re-exports) |
| File verdict | REAL (crate glue + docs; the crate is reachable and 86-test green; one S3: the effects re-export surface is a zero-consumer subsystem) |
| Reachable from shipped entry point? | YES — the crate itself: apps/cli/src/pipeline_host.rs:51 `use audio_engine::{AudioPipeline, PipelineSpec}` (+ AudioSink :619, DriftStats :671/1068) on the `lumen open` path; baseline AVI bench (60/60 audio frames) and sample-aac.mp4 Playing exercised it. The effects re-exports (line 123): NO consumer anywhere (see F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read lib.rs 1-128 · `rg -n "EffectChain\|AudioEffect\|Balance::new\|Gain::new" repo` (callers: only lib.rs re-export, effects.rs, effects/eq.rs + tests — zero production consumers; independently confirmed by audit/files/crates/audio-engine/src/effects.rs.audit.md [W07-A03] and effects/eq.rs.audit.md) · `rg -n "STAGE" crates/audio-engine/src/error.rs` (STAGE = PipelineStage::Mix — doc claim verified) · `rg -n "unwrap\(\)\|expect\(" crates/audio-engine/src/*.rs` (ZERO production unwraps — claim verified) · `rg -n "unsafe" crates/audio-engine/src` (none in code — only doc mentions) · `rg -c "Doc-tests" audit/runs/baseline/test-workspace.log.txt` (0 — doctests have NO baseline execution evidence) · `rg -n "doctest" Cargo.toml crates/audio-engine/Cargo.toml .cargo/config.toml` (not disabled) · baseline test-workspace.log.txt:41-43+ (audio-engine 86 unit tests green) · `sed -n` quote verification below |

## 1. One-paragraph verdict
The crate's front door is mostly honest and accurate: the four-stage description (A convert → volume/channel-map → C mix → D interleave) matches the module tree exactly; the sample-count-conservation section (29-45) states the wave-question answer precisely — rate-preserving by construction, the drift micro-resampler is the ONLY sanctioned exception, a rate-mismatched track stays a typed error "never a silent resample" (all verified in my resample/pipeline/mix audits); "no unwraps" is verified (zero production `unwrap()`/`expect(` in all 10 modules); "no unsafe" verified; the error-stage claim (Mix) matches error.rs's STAGE constant; and the stage-D sentence is honest that the integer saturating clamp is for "backends later cards add" — directly contradicting mix.rs's false "stage D's saturating clamp is where magnitude is bounded" (my mix.rs F-001: the clamp is dead code and the F32 path is unclamped). Two findings: the re-export line publishes the effects subsystem (EffectChain/Gain/Mute/Balance/MAX_EFFECTS) that has zero production consumers anywhere while the same file's docs say "insertion point B is empty in v0" — the export surface advertises a feature the pipeline never calls; and the doctest — the crate's canonical example and the only non-unit exercise of `set_volume` — has no execution evidence in the baseline (zero "Doc-tests" sections in the workspace test log despite doctests being enabled).

## 2. What actually works (verified)
— Baseline `test-workspace.log.txt:41-43` — `Running unittests src/lib.rs (audio_engine)` → 86 tests, 0 failed (incl. the zero-alloc suites at :144-151); the crate compiled in the release build (build-release-workspace.log.txt exit 0).
— Doc-claim vs code cross-checks (all by read): stage list ↔ module tree (`convert`/`volume`+`channel_map`/`mix`/`convert`); "stage Mix" ↔ error.rs:13 `const STAGE: PipelineStage = PipelineStage::Mix`; "insertion point B is empty" ↔ pipeline.rs imports (no effects use); "never a silent resample" ↔ pipeline.rs:288-290 typed `audio.rate_mismatch`; "DriftCorrection::ZERO restores the 1:1 path bit-exactly" ↔ pipeline test `zero_correction_is_the_bit_exact_bypass` (baseline green).
— "No panics, no unwraps" (51): zero production `unwrap()`/`expect(` (rg across all 10 modules; the only `unwrap_or` variants are total); `unsafe` count zero.
— Std-only (57): the crate's deps are the two in-tree path crates (media-types, media-clock) — no external deps (Cargo.toml verified in the wave-1 Cargo.toml audit).

## 3. Findings
### F-001 [S3]
Where: crates/audio-engine/src/lib.rs:123 (with 16-19)
Code:
```
pub use effects::{AudioEffect, Balance, EffectChain, Gain, MAX_EFFECTS, Mute};
```
What is wrong: this line publishes the effects subsystem on the crate's public API, but nothing outside effects.rs/effects/eq.rs and tests ever constructs any of it — my `rg` over the whole repo finds zero production consumers (the CLI imports only AudioPipeline/PipelineSpec/AudioSink/DriftStats; `pipeline.rs` never references effects), independently confirmed by W07-A03's effects.rs and eq.rs audits ("zero production callers anywhere"). The same lib.rs says at 16-19 that insertion point B "is empty in v0" — the docs tell the truth about the pipeline while the export surface implies a wired feature. The feature matrix even cites effects.rs as evidence that UI volume/mute are "live" (feature-matrix.toml:1667) — a false attribution W07-A03 already filed (the live path, where it exists, is the mixer).
Why it matters: the public API is the contract; exporting an unwired subsystem invites consumers (plugins, the tauri app, future hosts) to build against a dead end, and it inflates the crate's apparent capability — the exact "naming that lies" class this audit exists to catch.
What correct looks like: either gate the module (`#[doc(hidden)]`/`pub(crate)` + a doc note "unwired, insertion point B lands with card X") or wire `EffectChain` into `AudioPipeline` at stage B; the re-export should not ship ahead of the wiring.
Confidence: HIGH — Verification: read + repo-wide caller census + two independent prior audits agreeing.
### F-002 [S4]
Where: crates/audio-engine/src/lib.rs:63-108 (the doctest)
Code:
```
//! pipeline.process(0, &frame)?;
//! let had_output = pipeline.mix()?;
//! assert!(had_output);
//! let buffer = pipeline.take_output();
```
What is wrong: the doctest — the crate's only end-to-end usage example, and the only code outside unit tests that calls `AudioPipeline::set_volume` (a method with no production caller, per part1's finding) — has no execution evidence: `rg -c "Doc-tests" audit/runs/baseline/test-workspace.log.txt` → 0, and doctests are not disabled in any Cargo.toml/.cargo/config.toml I can find. Either the baseline log dropped the doctest sections (tooling/evidence gap) or doctests never ran; either way the crate's canonical example is UNVERIFIED-EXECUTED.
Why it matters: a doctest that silently rots (API drift would break it) while never running anywhere gives false confidence; the example is also the de-facto documentation for the set_volume flow.
What correct looks like: baseline evidence should include the Doc-tests section; if the sandbox harness excludes them, the INDEX should say so (REQUEST-ORCH-RUN below).
Confidence: HIGH (absence of evidence verified) — Verification: rg over the baseline log; UNVERIFIED whether the doctest passes (not run per available evidence).

## 4. Per-function table (items, not functions — the file is docs + glue)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs "What this crate is" | 1-27 | REAL | — | stage list matches module tree; stage-D sentence honest about later-card clamp (contradicts mix.rs:11-14 — see mix.rs F-001) |
| module docs "Sample-count conservation" | 29-45 | REAL | — | the wave-question's answer stated precisely and verified against pipeline/resample code + tests |
| module docs "Errors"/"Never" | 47-57 | REAL | — | stage Mix ↔ error.rs:13; no-unwrap/no-unsafe verified; std-only verified |
| doctest example | 59-108 | REAL-UNPROVEN | F-002 | compiles plausibly (uses public API correctly, asserts 16-byte output); no execution evidence in baseline |
| `pub mod buffer` … `volume` (10 decls) | 110-119 | REAL | — | all 10 module files exist; effects is the unwired one (F-001) |
| `pub use buffer::{AudioBuffer, AudioSink}` | 121 | REAL | — | AudioSink consumed at pipeline_host.rs:619 |
| `pub use channel_map::ChannelMap` | 122 | REAL | — | pipeline-internal + tests; no external consumer (API surface) |
| `pub use effects::{…}` | 123 | UNREACHABLE (production) | F-001 | zero consumers repo-wide |
| `pub use governor::{DriftGovernor, DriftStats}` | 124 | REAL (DriftStats) / UNREACHABLE (DriftGovernor construction) | — | DriftStats read at pipeline_host.rs:671/1068; DriftGovernor armed only by tests (resample.rs F-002 / part1 F-001) |
| `pub use mix::Mixer` | 125 | REAL | — | used by pipeline.rs:343 |
| `pub use pipeline::{AudioPipeline, PipelineSpec}` | 126 | REAL | — | the shipped entry (pipeline_host.rs:51,1600) |
| `pub use resample::MicroResampler` | 127 | UNREACHABLE (kernel) | — | construction reachable via pipeline; `process` dead in production (resample.rs F-002) |
| `pub use volume::Volume` | 128 | REAL | — | used by pipeline set_volume (API) + tests |

## 5. Test honesty
No tests in this file; the doctest is the test-like artifact (see F-002). The doctest's asserts (had_output true, 16 bytes) WOULD fail on a pipeline regression (wrong interleave size, dropped quantum) if it ever runs.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Stage A/C/D descriptions + "insertion point B is empty in v0" | lib.rs 9-23 | REAL | module tree + pipeline imports verified |
| "integer targets are converted with saturating clamp for the backends later cards add" | lib.rs 22-23 | REAL (honest) — and it REFUTES mix.rs:11-14's claim that the clamp bounds magnitude today | convert.rs:283-318 clamp exists, zero production callers (mix.rs F-001) |
| "Sample counts … map 1:1 to device samples … the micro-resampler is the one sanctioned exception … ≤ ±1000 ppm" | lib.rs 29-41 (quoting doc 03 §4.8 e) | REAL | ratio tests green; but the exception is UNREACHABLE in production (governor never armed — resample.rs F-002) |
| "a track whose rate differs from the negotiated mixer rate remains a typed Unsupported error, never a silent resample" | lib.rs 42-45 | REAL | pipeline.rs:288-290 + test 652-666 green — the definitive wave-question answer: no sample-rate conversion exists |
| "No panics, no unwraps" | lib.rs 51 | REAL (production paths) | rg: zero unwraps; panic paths found by this wave are defensive/unreachable-by-caller-invariant (resample.rs F-001, ms.rs F-001) |
| "Std-only; no `unsafe`" | lib.rs 57 | REAL | rg: no unsafe; deps are in-tree path crates |
| effects "live" via ui settings | feature-matrix.toml:1667 (cites effects.rs) | FALSE (attribution) | live path is the mixer; effects unwired (F-001; W07-A03's effects.rs audit F-001/F-002) |

## 7. Dead / duplicated / unreachable code
- `pub use effects::{…}` (123) — the unwired subsystem's front door (F-001); see also `pub mod effects` (113).
- `MicroResampler` re-export (127): type is real; its kernel unreachable in production.
- `DriftGovernor` (124): constructible, armed only by tests.
- No duplication within this file.

## 8. Dependencies & flags
- Deps: media-types + media-clock (in-tree path deps) + testkit (dev) — matches the manifest and the "std-only" claim; no feature flags on this crate; no external deps.
- All 10 modules compile in the default build (baseline build exit 0); no cfg-gating anywhere in the crate.

## 9. Open questions for the next agent
- Whoever owns the baseline tooling: why do no Doc-tests sections appear in test-workspace.log.txt? REQUEST-ORCH-RUN: `cargo test -p audio-engine --doc --locked` to establish whether lib.rs's example actually passes.
- The effects wiring card owner: F-001's fix (wire-or-gate) should be decided once for effects.rs + eq.rs + this re-export line together.
