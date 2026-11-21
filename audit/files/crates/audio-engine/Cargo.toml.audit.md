# AUDIT: crates/audio-engine/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 38 / 9 config keys (package×6, lints, dependencies×2, dev-dependencies×1) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. Call chains: apps/cli/Cargo.toml:48 `audio-engine = { path = … }` (direct), and apps/cli:49 → crates/audio-output/Cargo.toml:33 → audio-engine. Runtime: baseline engine-info reports feature `audio.null-device` and the AVI first-vertical smoke decodes/delivers 60 audio frames (mp4-smoke-bench-avi-mjpeg-pcm.txt: `a_frames=60 a_decoded=60`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Grep `audio-engine` in **/Cargo.toml (dependents: apps/cli:48, apps/web:67, audio-output:33) · grep `media_clock` in src/ (governor.rs:57-58, pipeline.rs:24, resample.rs:56 — real uses) · grep `media_types` in src/ (10 files) · grep `testkit` in tests/ (2 of 3 files) · sed -n boundaries.toml [crates.audio-engine] (exact match) · `grep -n "pub mod\|pub use" lib.rs` (11 modules, 8 export groups) · grep `#\[test\]` in tests/ (6 tests / 3 files) |

## 1. One-paragraph verdict
The cleanest manifest in my batch: two production deps both used in real
code (media-clock in the drift governor/pipeline/micro-resampler — the
"pre-noted edge" comment is accurate), one dev-dep used, zero external deps
(C5 true), boundaries.toml agreement exact, no features, and — unlike
transcode/platform-windows — REACHABLE from the shipped CLI both directly and
through audio-output. The single defect is metadata lag: the package
description still sells the 2.C3 "v0" surface (volume, channel map, straight
mix) while lib.rs now exports effect chains, a drift governor, a
micro-resampler, and a full pipeline.

## 2. What actually works (verified)
— `media-clock` dep REAL: governor.rs:57-58
`use media_clock::sync::{DriftEstimator, RateWindow};` +
`use media_clock::{DriftCorrection, MAX_CORRECTION_PPM};`, pipeline.rs:24,
resample.rs:56 — the manifest's dep comment (lines 27-32) describes exactly
these symbols, accurately.
— `media-types` dep REAL: 10 of 11 src files reference it.
— dev-dep `testkit` used: zero_alloc_quantum.rs, zero_alloc_resample.rs.
— Boundaries agreement: boundaries.toml `[crates.audio-engine]` →
internal_deps = ["media-types", "media-clock"], dev_deps = ["testkit"] —
identical.
— C5 claim TRUE: zero external crates.
— Registry-vs-allowlist honesty: registry row (doc 01 §2.2) names plugin-api
as a third dep; manifest documents it as a future edge ("the AudioFilter
plugin seam joins with the filter-chain card") — consistent with
boundaries.toml, which omits it too.
— Crate builds + all 6 integration tests pass: baseline #1 exit 0; baseline
#3 lists exactly one workspace failure (apps/desktop), so audio-engine's
zero-alloc tests passed.

## 3. Findings
### F-001 [S4] — package description lags the crate surface by three cards
Where: crates/audio-engine/Cargo.toml:19
Code:
```toml
description = "Lumen audio path v0: volume, channel map, straight mix — decoded AudioFrames to the mixer format and out to the sink (doc 03 §4.8)."
```
What is wrong: lib.rs:121-128 exports `AudioEffect, Balance, EffectChain,
Gain, Mute`, `DriftGovernor, DriftStats`, `MicroResampler`,
`AudioPipeline, PipelineSpec` — none of which the description mentions. The
header comment (lines 5-13) has the same lag: "This card's v0 surface —
volume, channel map, straight mix … needs media-types only", immediately
followed by a [dependencies] block that (correctly, per its own comment)
adds media-clock for the 3.B1 drift-resampler join.
Why it matters: The description is the crate's one-line identity in
`cargo metadata` / dependency trees; auditors and future maintainers size
the crate by it. A description three cards stale invites exactly the
doc-vs-code drift the devil audit exists to catch.
What correct looks like: "Lumen audio path: volume, channel map, straight
mix, effect chains, drift-governed micro-resampling, pipeline to the sink".
Confidence: HIGH — Verification: read (lib.rs:121-128 export list).

## 4. Per-key table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `package.name = "audio-engine"` | 16 | REAL | — | Matches boundaries.toml |
| `package.version = "0.1.0"` | 17 | REAL | — | Repo convention |
| `package.edition = "2024"` | 18 | REAL | — | Workspace-consistent |
| `package.description` | 19 | REAL | S4 | Stale vs lib.rs exports (F-001) |
| `package.publish = false` | 20 | REAL | — | — |
| `[lints] workspace = true` | 22-23 | REAL | — | Inherits deny tables; crate has no unsafe (hygiene counts attribute unsafe to platform-FFI + codecs hw crates) |
| `media-types` dep | 26 | REAL | — | Used in 10 src files |
| `media-clock` dep | 33 | REAL | — | Used (governor.rs:57-58, pipeline.rs:24, resample.rs:56); dep comment accurately names the symbols; matches boundaries |
| dev-dep `testkit` | 38 | REAL | — | Used in 2 of 3 test files |
| (absent `[features]`) | — | REAL | — | No features → nothing for release.yml to skip (hostile question N/A here) |

## 5. Test honesty
No manifest tests. The crate's 6 integration tests (2 per file:
audio_bench.rs, zero_alloc_quantum.rs, zero_alloc_resample.rs) ran clean in
baseline #3. The zero-alloc pattern is demonstrably non-vacuous: its sibling
in apps/desktop CAUGHT a real regression in the same run (608 bytes
allocated → 1 failed test, INDEX.md #3). Would audio-engine's variants fail
if allocation regressed? Same harness shape → yes by construction; runtime
proof is the src auditor's scope.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This card's v0 surface … needs media-types only" | Cargo.toml:6-7 | REAL (historical) / stale as description of current deps | Dep block adds media-clock with its own accurate 3.B1 justification (lines 27-33) |
| "media-clock joins with the drift micro-resampler (doc 03 §4.8 e)" | Cargo.toml:12-13 | REAL | resample.rs:56 `use media_clock::DriftCorrection;` + governor.rs:57-58 |
| "the AudioFilter plugin seam joins with the filter-chain card" (plugin-api absent) | Cargo.toml:9-11 | REAL-UNPROVEN | No plugin-api dep today (boundaries agrees); the filter-chain card is future work |
| "Std-only by C5: zero external dependencies" | Cargo.toml:13 | REAL | [dependencies] = 2 path deps only |
| "No clock objects cross this edge — the pipeline stores a correction, never a clock" | Cargo.toml:31-32 | REAL-UNPROVEN (src auditor) | pipeline.rs:24 imports only DriftCorrection (a value type), consistent |

## 7. Dead / duplicated / unreachable code
None. Reachable (apps/cli direct + via audio-output). No unused/duplicated
deps. No features.

## 8. Dependencies & flags
2 internal prod deps + 1 internal dev dep; zero external; zero features;
`[lints] workspace = true`. boundaries.toml agreement exact. The registry's
third dep (plugin-api) is a documented future edge, absent from both
manifest and boundaries — consistent.

## 9. Open questions for the next agent
- Src auditor: confirm no `Clock`-type object crosses the media-clock edge
  (manifest promises "a correction, never a clock" — pipeline.rs/governor.rs
  to prove).
- If the filter-chain card lands plugin-api, update this manifest AND the
  description (F-001) in the same change.
