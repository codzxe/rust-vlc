# AUDIT: crates/input/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-21 |
| Lines / config keys | 38 / 8 (package 5, lints 1, features 1, deps 2, dev-deps 1) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/Cargo.toml:54 `input = { path = … }` → apps/cli/src/demux_host.rs:394,419,422 `input::FileAccess::new()` / `input::resolve_file()` — the shipped file-open path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `^input = ` over member manifests (10 dependents: apps/cli, demux, playback-engine, media-probe, codecs, streaming, transcode, mem-audit, sweep + bench-vs-vlc with `contract`); rg `input::|use input` apps/cli/src (demux_host.rs:394,419,422 runtime call sites); rg cfg gating (lib.rs:64-65 `#[cfg(any(test, feature = "contract"))] pub mod contract;`); rg `unwrap_or|unwrap_none` in contract.rs (27 — explains hygiene-perfile's regex count with zero exact `.unwrap()`, per hygiene-counts.txt exact-literal list); sed boundaries.toml:161-166 (edge match); rg "network|Tcp|Udp|http" claims vs src file list (none) |

## 1. One-paragraph verdict
Clean and verified: both internal deps are used, the crate sits under the shipped CLI's real file-open path, the boundaries entry matches the manifest key-for-key, and the one feature (`contract`) is exactly the documented test-only gate — the cfg attribute at lib.rs:64-65 matches the manifest's claim word for word, and only a standalone harness (tools/bench-vs-vlc) enables it. The "no unwrap/expect/panic" claim survives a hostile reading of the hygiene evidence: contract.rs's 28 regex matches are `unwrap_or*` variants, not `.unwrap()`, and the module is compiled out of release binaries anyway. Zero findings.

## 2. What actually works (verified)
- Reachability: apps/cli/Cargo.toml:54 path-dep; runtime call sites apps/cli/src/demux_host.rs:394 (`input::FileAccess::new()`), :419 (`input::resolve_file(&parsed)`) — plus 9 other workspace dependents (demux, playback-engine, media-probe, codecs, streaming, transcode, mem-audit, sweep).
- Dep usage: media-types (SourceUri, per lib.rs:3 doc + uri.rs), plugin-api (`plugin_api::Access` implemented by FileAccess, lib.rs:14).
- Feature honesty: `contract = []` gates `src/contract.rs`; lib.rs:64-65 `#[cfg(any(test, feature = "contract"))]` — release binaries carry none of it; sole external enabler is tools/bench-vs-vlc/Cargo.toml:52 (`features = ["contract"]`), a standalone harness.
- Boundary conformance: boundaries.toml:161-166 internal_deps=[media-types, plugin-api], dev_deps=[testkit], no externals — exact match; baseline #11: no input violation.
- Std-only claim: no externals in any table; src modules (buffered, file, seekable, uri, error) contain no network code — "Network protocols belong to streaming — never here" holds.
- Lint claims: baseline #5 clippy `-D warnings` CLEAN across the workspace (which compiles input); exact `.unwrap()` count for input/src/contract.rs is zero (hygiene-counts.txt's 15 exact matches live only in tools/vta + xtask + lint_canary).

## 3. Findings
None. Checked and clean: dependency usage (2/2 internal + testkit), feature gating (verified cfg attribute + sole enabler), boundaries match, reachability (CLI runtime call sites), std-only/no-network claim, no-unwraps-in-shipped-code claim, publish=false, edition 2024, no dev-dep leakage.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition/description/publish | 14-19 | REAL | — | description accurate vs src layout |
| [lints] workspace = true | 21-22 | REAL | — | enforced (baseline #5) |
| [features] contract = [] | 24-30 | REAL | — | test/conformance-only; cfg-gated at lib.rs:64-65; enabled only by tools/bench-vs-vlc:52 |
| [dependencies] media-types | 33 | REAL | — | used (SourceUri resolution, uri.rs) |
| [dependencies] plugin-api | 34 | REAL | — | used (`plugin_api::Access`, lib.rs:14) |
| [dev-dependencies] testkit | 36-38 | REAL | — | dev-only; matches boundaries dev_deps |

## 5. Test honesty
No tests in the manifest. Crate tests ran in baseline #3/#4; input not among failures. The contract module's conformance tests compile only under test/feature — meaning `cargo test --workspace` (no features) exercises them via cfg(test); `--all-features` (baseline #4) also compiled them. Consistent.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Implements plugin_api::Access for local sources: single files first" | Cargo.toml:4-5 | REAL | lib.rs:14 FileAccess; apps/cli/src/demux_host.rs:394 uses it |
| "Network protocols belong to streaming (Phase 6) — never here" | Cargo.toml:6-7 | REAL | src modules are file/buffer/uri/seekable only (ls crates/input/src) |
| "C5 discipline: std-only, zero external dependencies" | Cargo.toml:9 | REAL | no external deps in any table |
| "No unsafe, no unwrap/expect/panic (workspace lints)" | Cargo.toml:9-10 | REAL | workspace lints deny + baseline #5 CLEAN; exact `.unwrap()` in input = 0 (hygiene-counts.txt) |
| "the reader's fixed-capacity buffer plus … one bounded read request" (MP §12 bounded memory) | Cargo.toml:10-12 | REAL-UNPROVEN | manifest-level: consistent with module names (buffered.rs); buffer-capacity arithmetic is the src auditor's file |
| contract module "test/conformance surface only … Release binaries (no feature) carry zero of its panicky conformance vocabulary" | Cargo.toml:25-29 | REAL | lib.rs:64-65 cfg gate verified; sole enabler is a non-shipped harness |

## 7. Dead / duplicated / unreachable code
None at manifest level. The `contract` feature is unused inside the shipped workspace (only the standalone bench-vs-vlc harness enables it) — by design and documented; the "future doc 06 §7.2 conformance kit" consumer does not exist yet (note, not a defect).

## 8. Dependencies & flags
2 internal deps, 1 dev dep, 0 externals, 1 feature (test-only). All boundary-declared. No version drift possible (path deps only).

## 9. Open questions for the next agent
- The src auditor for crates/input/src/buffered.rs should verify the fixed-capacity claim with actual capacity constants (MP §12 bounded-memory claim is only structurally verified here).
- Why do 10 crates need a direct `input` edge (codecs → input? transcode → input?) — layering breadth is the arch agent's question; check-architecture currently allows all of them.
