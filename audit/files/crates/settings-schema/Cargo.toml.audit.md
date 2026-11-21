# AUDIT: crates/settings-schema/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-21 |
| Lines / config keys | 43 / 9 (package 5, lints 1, deps 3, bin 2) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/Cargo.toml:61 `settings-schema = { path = … }` → apps/cli/src/host.rs:1402 `settings_schema::runtime::validate_value` on the set-setting path; apps/cli/src/help.rs:34 `use settings_schema::{GenTarget, generate_str}`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg dependents over member manifests (apps/cli + crates/{commands,plugin-api,playlists,plugin-host-wasm,apps/control-server} reference it; apps/cli is the path-dep); rg usage in apps/cli/src (help.rs:34,157; host.rs:1371,1400,1402); rg serde/toml/thiserror in crates/settings-schema/src (all used: parse.rs:15 `use serde::Deserialize`, lib.rs:67 `#[derive(Debug, thiserror::Error)]`, lib.rs:117 `toml::from_str`); ls schemas/ + schemas/generated/ (settings.toml + 4 artifacts exist); rg include_str (runtime.rs:18 embeds ../../../schemas/settings.toml); git log/show 172bcdbb (enum "+auto" change) + content check of all three generated artifacts for "auto" (help.txt:143 ✓, docs.md:99 ✓, d.ts carries key names only — no drift); sed tools/xtask/src/cmd/schema_gen.rs (subprocess bridge) + tools/xtask/Cargo.toml (zero-dep policy consistent); rg boundaries.toml:104-109 (externals match); rg .github/workflows for schema-gen --check (ci.yml:278) |

## 1. One-paragraph verdict
Clean manifest, fully verified end to end: the three external deps are all used, the embedded schema path exists, the crate is a real dependency of the shipped CLI (runtime validation of `config.set-setting` at host.rs:1402 — the G4 claim is true), the subprocess bridge it describes in xtask exists exactly as stated, the boundaries entry matches, and the generated artifacts are current — including the `interface.theme` enum "+auto" change of 2026-11-13, which I chased through git history and found correctly reflected in settings-help.txt:143, settings-docs.md:99, and ui/src/screens/settings/generated/schema.ts:96 (settings.d.ts carries key names, not enum values, so its older commit date is not drift). Zero findings; everything below documents what was checked.

## 2. What actually works (verified)
- Reachability: apps/cli/Cargo.toml:61 path-dep; apps/cli/src/host.rs:1402 `settings_schema::runtime::validate_value(&full, &toml_value)` — set-setting writes validated at runtime in the shipped binary (G4 claim TRUE).
- Dep usage: serde (parse.rs:15,70-112 Deserialize derives), toml (lib.rs:117,163 `toml::from_str::<toml::Table>`), thiserror (lib.rs:67) — all three externals used.
- G1-G3 artifacts exist and are current: schemas/generated/{settings.d.ts, settings-help.txt, settings-docs.md} + commands.checksum.txt; the 2026-11-13 `enumeration = ["dark", "light", "auto"]` change (git show 172bcdbb) is present in help.txt (`--interface-theme <dark|light|auto>`) and docs.md (`dark, light, auto`).
- Subprocess bridge claim: tools/xtask/src/cmd/schema_gen.rs:66-77 runs `cargo run -p settings-schema --bin schema-gen` — matches "the entry point the (frozen) xtask schema-gen subcommand shells out to"; xtask's own manifest confirms the zero-dep freeze policy (tools/xtask/Cargo.toml:9-13).
- Boundary conformance: boundaries.toml:104-109 external_allowlist=[serde, toml, thiserror], internal_deps=[] — matches.
- CI enforcement: ci.yml:278 `cargo run -p xtask --locked -- schema-gen --check` — drift detection exists (not in baseline run set; see §9).
- G4 embedded source: runtime.rs:18 `include_str!("../../../schemas/settings.toml")` resolves (file exists).

## 3. Findings
None. Checked and clean: dependency usage (3/3 used), lints inheritance (workspace = true, baseline #5 clippy clean), boundaries match, bin path exists (src/bin/schema_gen.rs), generated-artifact freshness (git-history chase described in §2), version-family claim ("same family product-config uses" — product-config/Cargo.toml:21-24 has identical req strings "1"/"1"/"2"), reachability (CLI path-dep + two call sites), publish=false, edition 2024.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition/description/publish | 26-31 | REAL | — | description accurate (verified against src layout) |
| [lints] workspace = true | 33-34 | REAL | — | enforced per baseline #5 |
| [dependencies] serde (derive) | 37 | REAL | — | used: parse.rs Deserialize derives |
| [dependencies] toml = "1" | 38 | REAL | — | used: lib.rs:117,163 |
| [dependencies] thiserror = "2" | 39 | REAL | — | used: lib.rs:67 |
| [[bin]] schema-gen | 41-43 | REAL | — | src/bin/schema_gen.rs exists; xtask bridge shells to it (schema_gen.rs:66-77); CI --check at ci.yml:278 |

## 5. Test honesty
No tests in the manifest. Crate tests ran in baseline #3/#4 (workspace runs); settings-schema not among the failures.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "G4 (runtime validation) is this crate's library … config.set-setting-style writes are validated against it (type/range/enum — never silent coercion)" | Cargo.toml:10-12 | REAL | apps/cli/src/host.rs:1402 validate_value call site |
| "Regeneration is idempotent (cargo xtask schema-gen, twice → empty git diff)" | Cargo.toml:14-15 | REAL-UNPROVEN | cannot run cargo (rule 8); CI --check (ci.yml:278) enforces the drift half |
| "the schema-gen binary is the entry point the (frozen) xtask schema-gen subcommand shells out to" | Cargo.toml:17-20 | REAL | tools/xtask/src/cmd/schema_gen.rs:66-77 |
| "serde 1.x, toml 1.x, thiserror 2.x (same family product-config uses). No new dependencies." | Cargo.toml:22-24 | REAL | deps exactly those three; product-config/Cargo.toml:21-24 identical reqs |
| Generates the three committed artifacts | Cargo.toml:6-9 | REAL | all three exist in schemas/generated/ and are current (incl. +auto enum, §2) |

## 7. Dead / duplicated / unreachable code
None in this manifest. (Note for scale: other consumers found — commands, plugin-api, playlists, plugin-host-wasm, control-server — the crate is a shared foundation, not an orphan.)

## 8. Dependencies & flags
3 externals, all boundary-allowlisted and used; no features beyond serde derive; no dev-deps (tests use std only — consistent with the crate's std-only-adjacent posture). Version-requirement drift vs plugin-api (`toml "1"` here vs `"1.1"` there) is recorded as root-manifest finding F-002, not a per-file defect.

## 9. Open questions for the next agent
- `schema-gen --check` was not in the baseline run set (only ci.yml declares it) — the orchestrator may want it in a future gate run to prove zero drift today.
- ui/src/screens/settings/generated/schema.ts is generated by ANOTHER path (ui tokens:check family per baseline #24) — someone should confirm both generators consume the same schemas/settings.toml and cannot fork.
