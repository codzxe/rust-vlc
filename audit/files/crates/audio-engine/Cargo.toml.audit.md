# AUDIT: crates/audio-engine/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 38 / 4 config keys ([package] name/version/edition/description/publish, [lints], [dependencies]×2, [dev-dependencies]×1) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — audio-engine is a release-lane dependency of the shipped CLI (apps/cli constructs `AudioPipeline`, pipeline_host.rs:51) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | cross-checks: `rg` of boundaries.toml `[crates.audio-engine]` row, doc 01 §2.2 row 239, and imports in src/ (all deps used) |

## 1. One-paragraph verdict
Minimal, honest manifest: exactly two path dependencies (media-types, media-clock), one dev-dependency (testkit), zero external crates — matching boundaries.toml's `[crates.audio-engine]` row verbatim (`internal_deps = ["media-types", "media-clock"]`, `dev_deps = ["testkit"]`, `external_allowlist = []`), so the "Std-only by C5: zero external dependencies" claim is true and machine-checked by the boundary manifest. Both dependencies are genuinely used (`media_types::frame/…` throughout, `media_clock::{DriftCorrection, DriftGovernor}` in pipeline.rs/governor wiring). No `[features]` section — nothing in this crate is feature-gated, so the audited pipeline path is unconditionally compiled into every build that links the crate. The single discrepancy is documentation-level: doc 01 §2.2's registry row lists `plugin-api` as a dependency the manifest does not carry — disclosed in this file's own header comment ("the manifest is an allowlist, the registry row's fuller dep list lands with the cards that need it"), with the referenced `audio_engine::AudioFilter` trait not yet existing anywhere in src/ (plugin-api/src/filter.rs:4-6 describes it as a future L3 domain trait).

## 2. What actually works (verified)
— Verified by cross-read: boundaries.toml `[crates.audio-engine]` (internal_deps/dev_deps/external_allowlist) == this manifest; `rg media_clock crates/audio-engine/src` → pipeline.rs + governor wiring; `rg media_types` → all modules; `rg testkit` → tests only. No unused dependencies, no undeclared dependencies.

## 3. Findings
F-001 [S4]
Where: crates/audio-engine/Cargo.toml:4-5 vs docs/01-architecture.md:239
Code:
```
# Registry: doc 01 §2.2 (`audio-engine`, layer L3, deps: media-types,
# media-clock, plugin-api). This card's v0 surface — volume, channel
```
What is wrong: doc 01 §2.2's row (`| audio-engine | crate | L3 | media-types, media-clock, plugin-api | 2 | audio filters |`) claims a plugin-api dependency that the manifest does not have and that nothing in src/ uses (no `plugin_api` import; the `AudioFilter` trait it would anchor does not exist yet). The header comment pre-discloses the gap, so this is a documented doc-vs-manifest drift, not a hidden one — but a reader of doc 01 alone believes audio filters are wired. What correct looks like: doc 01 row marked "planned" until the filter-chain card lands. Confidence: HIGH — Verification: read (rg both directions).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] | 15-20 | REAL | — | edition 2024, publish=false, honest description |
| [lints] workspace = true | 22-23 | REAL | — | inherits workspace lints (clippy -D warnings clean per baseline) |
| [dependencies] media-types | 26 | REAL | — | path dep, used |
| [dependencies] media-clock | 33 | REAL | — | path dep, used (DriftCorrection/DriftGovernor); edge documented ("No clock objects cross this edge — the pipeline stores a correction, never a clock") |
| [dev-dependencies] testkit | 35-38 | REAL | — | Must + alloc_count; dev-only as MP §3.7 requires |
| (absent) [features] | — | REAL | — | no feature gates — audited code unconditionally compiled |

## 5. Test honesty
No tests in this file. The crate's tests (lib 86 + zero_alloc_quantum + zero_alloc_resample + audio_bench) all resolve through these deps and were exercised in the sibling audits (pipeline: 14 passed; zero_alloc_quantum: 1 passed both thread modes).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: zero external dependencies" (L13) | Cargo.toml | REAL | external_allowlist = [] (boundaries.toml:362-363); both deps are path deps |
| "deps: media-types, media-clock, plugin-api" (L4-5, quoting doc 01) | Cargo.toml + doc 01:239 | FALSE for plugin-api today | no plugin_api import anywhere in crates/audio-engine/src (rg); disclosed in the comment itself |
| "the AudioFilter plugin seam joins with the filter-chain card" (L9-10) | Cargo.toml | REAL (as a plan) | plugin-api/src/filter.rs:4-6 stages the trait as [PROPOSAL]; audio-engine has no AudioFilter |

## 7. Dead / duplicated / unreachable code
None. (Note for context: `pub mod effects` (lib.rs:113) with effects.rs/eq.rs is exported API not consumed by the shipped hosts — the filter-chain future — but that is lib.rs's business, not this manifest's.)

## 8. Dependencies & flags
media-types (path), media-clock (path), testkit (dev, path). No features, no target-specific deps, no workspace-inherited deps hidden. Matches the boundary manifest exactly.

## 9. Open questions for the next agent
- The filter-chain card: when `AudioFilter`/plugin-api lands, the doc 01 row becomes true — until then doc 01:239 overstates. Who owns reconciling doc 01 rows with boundaries.toml (xtask arch check?)?
- audio-output's Cargo.toml (sibling crate): confirm it lists audio-engine + media-clock + media-types exactly as boundaries.toml:410 requires — it was outside this assignment.
