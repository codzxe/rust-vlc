# AUDIT: tests/compat/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / keys | 75 / 1 workspace table, 1 package, 2 lint tables, 8 deps |
| File verdict | REAL — standalone-workspace test tool manifest, honest header, deps justified |
| Reachable from shipped entry point? | NO — test tooling, deliberately outside the root workspace |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; rg consumers of declared deps in tests/compat/src |

## 1. One-paragraph verdict
A deliberate, documented standalone-workspace manifest for the
compat-matrix runner. The header explains the exact reason (root manifest
membership is Lead-frozen; the bench-vs-vlc precedent). The dependency
list is honest and minimal (lumen-cli + commands + codec-api +
media-types + plugin-api + playback-engine + bench-vs-vlc + serde +
serde_json), and it carries an in-manifest audit trail: the T5 (2026-11-17)
note records that codecs/demux/input/media-probe rows were dead
(cargo-machete) and removed because the runner reaches them through
lumen-cli's public modules.

## 2. What actually works (verified)
- Lint parity with the root policy (29-42): unsafe_code/unwrap_used/expect_used/panic/todo/unimplemented all deny — matches the header claim "mirrors the root lint policy locally".
- Deps-are-used spot check (rg in tests/compat/src — every path dep is imported):
  - decode.rs:29-37 imports codec_api, commands::catalog::types, lumen_cli::demux_host::open_source, media_types (3 rows), playback_engine::DecoderProvider, plugin_api::DemuxEvent
  - oracle.rs:249,264,510,552 use bench_vs_vlc::md5::Md5 (the reused RFC-1321 MD5)
  - serde/serde_json: config/rows/reports JSON (tests/compat/compat-matrix.json consumed via these)
- Standalone `[workspace]` (27) — root `cargo test --workspace` does not include this package (consistent with my baseline observation in the sha256.rs audit).

## 3. Findings
None. Zero-findings justification: no unused-dep rows remain (the T5 note
records the cleanup; remaining deps spot-verified used or named), no
version pins drift (serde "1", edition 2024 matches root rust-toolchain
1.99.1 era), publish = false, lints mirror root discipline, and the
membership workaround is documented rather than smuggled.

## 4. Per-function table (manifest keys)
| Key block | Lines | Verdict | Notes |
|---|---|---|---|
| header rationale | 1-25 | REAL (accurate) | cross-checked: root workspace excludes tests/compat; bench-vs-vlc precedent exists |
| [workspace] + lints | 27-42 | REAL | deny-set mirrors root policy |
| [package] | 44-52 | REAL | compat-matrix 0.1.0, publish=false, honest description |
| [dependencies] | 54-75 | REAL | 8 rows, each justified in comments; T5 audit note records the dead-row removal |

## 5. Test honesty
N/A (manifest).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "serde + serde_json ONLY ... no other external is admitted" | :20-25 | REAL | dep list = those two + path deps only |
| "codecs/demux/input/media-probe were declared here but never imported ... the direct rows were dead (cargo-machete)" | :61-65 | REAL-UNPROVEN | removal visible (rows absent); the historical deadness claim not re-runnable (needs cargo-machete) |
| "this crate mirrors the root lint policy locally" | :16-18 | REAL | lint tables match root Cargo.toml's policy (deny unwrap/expect/panic/todo etc.) |

## 7. Dead / duplicated code
None in the manifest itself.

## 8. Dependencies & flags
Path deps on 7 in-tree crates/tools + serde/serde_json. No features. Standalone workspace (empty [workspace]).

## 9. Open questions
- Confirm with the CI auditor whether .github/workflows/compat-matrix.yml gates `cargo test` for this package (see sha256.rs audit F-001).
