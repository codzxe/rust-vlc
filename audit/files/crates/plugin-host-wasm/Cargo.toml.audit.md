# AUDIT: crates/plugin-host-wasm/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-30 |
| Lines / functions | 83 / 4 prod deps, 6 dev-deps, 2 bins, 1 lints key |
| File verdict | REAL — manifest matches the code and the lockfile; every dependency is used; honest comments |
| Reachable from shipped entry point? | NO — crate is a workspace member (crates/*) but no app depends on it; release lane never builds it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 Cargo.lock closure analysis (quoted below); rg usage sweeps per dependency |

## 1. One-paragraph verdict
A clean manifest whose heavy comment load survives verification: the declared edges (plugin-api, settings-schema, toml, wasmi) all have real call sites, all six dev-dependencies are used by the crash-isolation composition suite, the wasmi feature posture matches the lockfile (8-crate closure), and the forward-looking edges the header withholds (media-types, commands) are indeed absent from the code. The costs: a 13-crate toml closure enters the production graph for exactly one constructor call, and two dev-tool binaries ride the crate that nothing ships.

## 2. What actually works (verified)
— verified by python3 over Cargo.lock: `wasmi deps: ['spin', 'wasmi_collections', 'wasmi_core', 'wasmi_ir', 'wasmparser']` → with wasmi itself + hashbrown + libm = the claimed 8-crate graph; `toml closure: [...13 crates...]` (F-002).
— verified by rg: `settings_schema::runtime` used at src/supervisor.rs:123,132,140,171; `toml::Value::Integer` at supervisor.rs:170 (the only toml call); dev-deps used at tests/plugin_crash_isolation.rs:30-45 (`use playback_engine::`, `codec_api::`, `media_clock::`, `packet_queue::`, `use testkit::Must`).
— Build/test evidence: this manifest produced the passing plugin-host-wasm suites I ran (imports.rs audit §2) and baseline cargo-test-workspace.txt:4884-5022 exit_code=0.

## 3. Findings
F-001 [S4]
Where: crates/plugin-host-wasm/Cargo.toml:73-83
Code:
```
[[bin]]
name = "lumenext"
path = "src/bin/lumenext.rs"
```
What is wrong: two dev-tooling binaries (`lumenext`, `lumenext-examples`) are bin targets of a library crate that no shipped product depends on — `cargo build --workspace` builds them on every dev machine, but no release lane or CI artifact ships them (release lane = lumen-cli + desktop only). Not a defect of the manifest per se; recorded so the wave's "shipped surface" census doesn't count them as product binaries. Confidence: HIGH — Verification: read + root Cargo.toml members + baseline release-build logs.

F-002 [S4]
Where: crates/plugin-host-wasm/Cargo.toml:55-57
Code:
```
toml = "1"

wasmi = { version = "2.0.0", default-features = false, features = ["std", "validate", "auto-dispatch"] }
```
What is wrong: `toml = "1"` admits an un-pinned minor (caret requirement; locked at 1.1.8+spec-1.1.0 today — any future 1.x is fair game, unlike wasmi whose ADR record is the version authority) and drags a 13-crate closure (toml, toml_parser, toml_writer, toml_datetime, serde_spanned, serde_core, serde_derive, indexmap, equivalent, winnow, proc-macro2, quote, unicode-ident) into the production graph for ONE call site (`toml::Value::Integer`, supervisor.rs:170). The comment justifies it as the settings-layer value type — the seam design forces it, but a `toml`-free value enum in settings-schema would remove 13 crates from this crate's graph. Today it ships nowhere (crate unembedded), so the cost is dev-build-only. Confidence: HIGH — Verification: python3 lockfile closure (quoted) + rg.

F-003 [S4]
Where: crates/plugin-host-wasm/Cargo.toml:16-24
Code:
```
# `auto-dispatch` is the 7.w22d repair: it is also a zero-crate
# feature (wasmi's own default) that selects the dispatch backend —
# without it the tail-call backend is compiled UNCONDITIONALLY, and
# LLVM does not perform the sibling-call optimization at opt-level 0/1,
# so every debug/test build (cargo test) stack-overflows on any guest
```
What is wrong: the stack-overflow history claim ("observed: the §9.4 fuel-loop fixture") is not reproducible without editing this manifest (forbidden — read-only audit); the feature is ON today and all fuel-loop tests pass (plugin_sandbox_security: 9 passed, including plugin_trap_loop). The claim is plausible and consistent, but the counterfactual is UNVERIFIED. Confidence: MEDIUM — Verification: read; counterfactual UNVERIFIED (would require a forbidden manifest edit + rebuild).

## 4. Per-key table (manifest, <100 LOC)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition/publish | 41-46 | REAL | — | edition 2024, publish=false, correct description |
| [lints] workspace = true | 48-49 | REAL | — | inherits the deny set; crate complies (mutex poisoning handled via `unwrap_or_else(|p| p.into_inner())`, state.rs) |
| dep plugin-api | 52 | REAL | — | path dep; the only edge doc 01 §2.8 v1-wires, as the header claims |
| dep settings-schema | 53 | REAL | — | used by supervisor.rs (config-driven crash policy) |
| dep toml | 55 | REAL | S4 | one call site, 13-crate closure (F-002) |
| dep wasmi | 57 | REAL | S4 | features match ADR-002 record; 8-crate graph verified; counterfactual UNVERIFIED (F-003) |
| [dev-dependencies] ×6 | 59-71 | REAL | — | all six used by tests (rg evidence in §2) |
| [[bin]] lumenext / lumenext-examples | 73-83 | REAL | S4 | dev tooling, unshipped (F-001) |

## 5. Test honesty
No tests in a manifest; the crate's suites built from this manifest all pass (imports.rs audit §2; baseline :4884-5022).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 wires the plugin-api manifest/permission contract only, so only that edge is declared" (media-types/commands forward-looking) | Cargo.toml:5-7 | REAL | `[dependencies]` has exactly plugin-api + settings-schema + toml + wasmi; no media-types/commands edge |
| "same 8-crate graph either way" (auto-dispatch adds no crate) | Cargo.toml:24 | REAL | lockfile: wasmi + spin + wasmi_collections + wasmi_core + wasmi_ir + wasmparser + hashbrown + libm |
| "the `validate` feature adds no crate" | Cargo.toml:13-14 | UNVERIFIED | feature-unification claim; verifying needs a build-graph diff (cargo-build-budget) |
| "no `wat`/`wast` dev-dependency" | Cargo.toml:30-31 | REAL | dev-deps list contains none; fixtures built by tests/common emitter |
| "7.F4 … settings-schema … the toml external joins only for the config.set-setting-shaped value type" | Cargo.toml:34-39 | REAL | single `toml::Value::Integer` call site (supervisor.rs:170) |

## 7. Dead / duplicated / unreachable code
No unused dependencies (every prod and dev dep has call sites — rare enough to state). The crate itself is unreachable from shipped builds (imports.rs F-001).

## 8. Dependencies & flags
Covered above; no features defined by this crate; no [[example]]/[[test]] custom targets beyond defaults.

## 9. Open questions for the next agent
- supervisor.rs (not my file): does `extensions.crash-*` config actually flow from schemas/settings.toml through `settings_schema::runtime::embedded()` — and is any shipped build ever configured to enable extensions at all?
- If the Lead ever embeds this crate in an app, F-002's 13-crate toml closure enters that app's shipped graph — re-flag at that point.
