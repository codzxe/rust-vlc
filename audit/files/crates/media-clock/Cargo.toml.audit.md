# AUDIT: crates/media-clock/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-21 |
| Lines / config keys | 27 / 7 (package 5, lints 1, deps 1, dev-deps 1) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/Cargo.toml:55 `media-clock = { path = … }` → apps/cli/src/pipeline_host.rs:67 `use media_clock::{MediaClock, PlaybackClock}` and :557/:613 `SampleCounterSource`/`DriftCorrection` — the shipped A/V clock) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `^media-clock = ` over member manifests (10 dependents: apps/cli, apps/desktop, apps/web, playback-engine, audio-engine, audio-output, video-engine, renderer-wgpu, subtitles(dev), plugin-host-wasm); rg `media_clock` apps/cli/src (lib.rs:83, pipeline_host.rs:67,557,613); rg media_types in src (5 files use it); rg testkit in src (all matches inside `#[cfg(test)] mod tests` — backends.rs:1014-1021, sync.rs:461-463 — or doc comments); rg -c exact `.unwrap()` in src (zero files match); sed boundaries.toml:172-177 (edge match) |

## 1. One-paragraph verdict
Minimal, clean, verified: one internal dep used everywhere it should be, one dev dep used only inside cfg(test) modules (the dev-edge discipline the header promises), zero externals, zero exact unwraps, boundaries entry matching key-for-key, and ten downstream dependents including the shipped CLI's pipeline host, where the clock actually runs. Zero findings.

## 2. What actually works (verified)
- Reachability: apps/cli/Cargo.toml:55; runtime sites apps/cli/src/pipeline_host.rs:67 (use), :557 (`Arc<dyn media_clock::SampleCounterSource>` — the injected sample counter the header describes), :613 (`observe_drift`); also apps/web/src/host.rs:91 and apps/desktop/Cargo.toml:54.
- Dep usage: media-types used in 5 of 7 src files (drift.rs, clock.rs, backends.rs, mapping.rs, source.rs).
- Dev-dep discipline: testkit appears in src only inside `#[cfg(test)] mod tests` (backends.rs:1014-1021, sync.rs:461-463) or doc comments (source.rs:8, clock.rs:837) — no leak into shipped code.
- Boundary conformance: boundaries.toml:172-177 internal_deps=[media-types], dev_deps=[testkit], no externals — exact match; baseline #11: no media-clock violation.
- Hygiene: exact `.unwrap()` count in crates/media-clock/src = 0 (rg -c: no file matches); the 44 regex matches in hygiene-perfile.txt for backends.rs are `unwrap_or*` variants (benign).
- Std-only + no-unsafe claim: no externals declared; workspace `unsafe_code = "deny"` enforced (baseline #5 CLEAN).

## 3. Findings
None. Checked and clean: dep usage, dev-dep isolation (cfg(test) verified), boundaries match, reachability (CLI runtime sites), std-only claim, no-exact-unwrap claim, description accuracy vs module set (backends/clock/drift/mapping/source/sync all named in the description's feature list), publish=false, edition 2024.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition/description/publish | 9-14 | REAL | — | description enumerates the actual module set (audio-master, backends, fallback, re-arm, mapping, pause, bounded drift) |
| [lints] workspace = true | 16-17 | REAL | — | enforced (baseline #5) |
| [dependencies] media-types | 19-20 | REAL | — | used in 5 src files |
| [dev-dependencies] testkit | 22-26 | REAL | — | used only in cfg(test) modules — the documented "virtual-time sources" posture |

## 5. Test honesty
No tests in the manifest. Crate tests ran in baseline #3/#4; media-clock not among failures. The `tests/zero_alloc_backends.rs` file aligns with the dev-dep comment (virtual-time/acceptance posture); its honesty is the src/test auditor's file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "deps: media-types only" (registry row) | Cargo.toml:3 | REAL | [dependencies] has exactly that |
| "never touches I/O, device APIs or UI" | Cargo.toml:5-6 | REAL | 7 src modules are pure timing logic; the audio device is an injected trait (pipeline_host.rs:557 `Arc<dyn SampleCounterSource>`) |
| "Std-only by C5: zero external dependencies, no unsafe" | Cargo.toml:7 | REAL | zero externals; unsafe_code deny inherited; baseline #5 CLEAN |
| "bounded drift correction" (description) | Cargo.toml:13 | REAL-UNPROVEN | module exists (drift.rs); boundedness arithmetic is the src auditor's file |

## 7. Dead / duplicated / unreachable code
None at manifest level. The crate is a workspace hub (10 dependents).

## 8. Dependencies & flags
1 internal dep, 1 dev dep, 0 externals, 0 features. All boundary-declared. No drift possible (path deps only).

## 9. Open questions for the next agent
- backends.rs is >1000 lines with a 500+ line test module inside — the src auditor should confirm the shipped half (lines ~1-1013) keeps the "deterministic master-loss fallback" honest, and that master-loss paths have tests that fail if the fallback order changes.
