# AUDIT: crates/audio-engine/Cargo.toml

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 38 / manifest (no functions) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — audio-engine is a dependency of the `lumen` CLI's audio path (apps/cli/Cargo.toml internal-deps comment names it; `open_audio_session` → AudioPipeline). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `cat` manifest (exit 0); cross-refs: `rg DriftEstimator|RateWindow` (governor.rs:57), sync.rs audit F-001. No cargo. |

## 1. One-paragraph verdict

An honest manifest. Dependencies are exactly the declared registry set (media-types, media-clock), std-only otherwise, testkit dev-only — all verified against actual imports (governor.rs:57 `use media_clock::sync::{DriftEstimator, RateWindow}`; convert.rs uses media-types frame types). The one substantive observation is that the media-clock dependency's stated purpose — "DriftCorrection … DriftEstimator/RateWindow (the pure sync primitives)" — is, in the shipped build, a dependency on code that never executes: the governor consuming those primitives is never armed in production (see my sync.rs audit F-001). The manifest is truthful about wiring intent; the wiring itself does not exist in any shipped host.

## 2. What actually works (verified)

- Dependency list matches imports: media-types (frame/error types in convert.rs, pipeline.rs), media-clock (governor.rs:57 only). No undeclared deps, no unused deps at the crate level (the crate compiles in the W00 release build with `--locked`).
- `testkit` is dev-dependency only — matches "dev-only by contract (MP §3.7)".
- The "No clock objects cross this edge — the pipeline stores a correction, never a clock" claim is consistent with what I read in pipeline.rs (governor stores DriftCorrection; no Arc<MediaClock> imported in audio-engine — `rg MediaClock crates/audio-engine/src` returns no hits).

## 3. Findings

### F-001 [S4] The media-clock dependency exists to serve a production-dead path
Where: crates/audio-engine/Cargo.toml:27-33
Code (verbatim):
```toml
# media-clock joins with card 3.B1's drift micro-resampler (doc 03
# §4.8 e) — the manifest's own pre-noted edge (boundaries.toml's
# audio-engine comment, written in 2.C3 for exactly this card):
# DriftCorrection (the ±1000 ppm limiter), DriftEstimator/RateWindow
# (the pure sync primitives). No clock objects cross this edge — the
# pipeline stores a correction, never a clock.
```
What is wrong: the dependency is real and used at compile time, but its only consumer (`DriftGovernor`, armed via `arm_drift_governor`) has no production caller (rg over apps/ + crates/: the sole call is apps/cli/tests/sync_scenarios.rs:93). The manifest documents the intended wiring, not the shipped wiring. Not a manifest defect — an entry in the claim ledger that "joins with card 3.B1" describes an aspiration that no host realizes.
Confidence: HIGH — Verification: rg caller search (documented in sync.rs audit F-001).

## 4. Per-function table

N/A (manifest). Keys checked: `[package]` name/version/edition/publish=false; `[lints] workspace = true`; `[features]` absent (no feature gates — everything compiles in the release build); `[dependencies]` 2 path deps; `[dev-dependencies]` 1.

## 5. Test honesty

N/A (no tests; the crate's tests live in src files audited separately — convert.rs audited above; mixer/pipeline/governor belong to other agents).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: zero external dependencies" | Cargo.toml:12 | REAL | only 2 path deps, both workspace-internal |
| "media-clock joins with card 3.B1's drift micro-resampler" | Cargo.toml:27 | SHALLOW | dependency used, but the resampler path is never armed in production (sync.rs audit F-001) |
| "No clock objects cross this edge" | Cargo.toml:32-33 | REAL | rg MediaClock over audio-engine/src: no hits |

## 7. Dead / duplicated / unreachable code

N/A. (The unreachable-code consequence is recorded in F-001 and the sync.rs audit.)

## 8. Dependencies & flags

No features. No external crates. `publish = false`. Workspace lints inherited (`[lints] workspace = true`) — meaning the crate is subject to the workspace's denied unwrap/expect/panic lints (claimed in input/Cargo.toml's C5 text; enforcement UNVERIFIED without clippy run).

## 9. Open questions for the next agent

- Does any doc/matrix row claim the drift micro-resampler as an operating feature? If yes, that row inherits the UNREACHABLE-armed verdict from sync.rs F-001 (waves 18/19).
