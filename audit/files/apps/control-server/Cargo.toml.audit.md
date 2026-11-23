# AUDIT: apps/control-server/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-23 |
| Lines / functions | 48 / 5 keys (package, lib, bin, dependencies, lints) |
| File verdict | REAL (accurate manifest; comments honest; one doc-01 registry drift) |
| Reachable from shipped entry point? | YES — builds `lumen-control-server` (bin) + `control_server` (lib); CI workspace build (ci.yml:233-234); NOT in the release lane (release.yml:193: `-p lumen-cli -p desktop`), and the product gate is OFF: `product.toml:167` `control-server = false # Phase 5+ [PROPOSAL]` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg dependency-usage census (all 5 deps used); `./target/debug/deps/control_server-58398a5795b79e6e` → 11 passed; `./target/debug/deps/e2e-2485aaf028bcffb2` → 6 passed; `./target/debug/deps/conformance-02b71ae06ac20e47` → 25 passed; read of boundaries.toml / product.toml / docs/01 |

## 1. One-paragraph verdict
The manifest is clean and matches its own comments: every declared dependency is used (rg census: `commands::` 33 uses, `control_protocol::` 34, `product_config::` 8, `serde_json::` 13, `toml::` 1 in config.rs), the externals (`serde_json`, `toml`) are exactly what boundaries.toml's `[crates.control-server]` section admits (L628-630), and the header comment honestly pre-discloses the v1 introspection-only surface ("the engine-embedding join … is the desktop composition card's; v1 serves the catalog-introspection subset"). The only drift is upstream of this file: doc 01 §2.8's registry row claims this app depends on "engine, contracts" — the manifest has neither, and no `contracts` crate exists in `crates/` at all. The product-model gate (`features.control-server = false`) and the release lane's exclusion make the whole app opt-in dev tooling, which the manifest comments don't spell out but boundaries.toml does.

## 2. What actually works (verified)
— Dependency census (rg): toml → config.rs:1 use; serde_json → logging.rs + e2e; product-config → main.rs/config.rs; control-protocol and commands used across all modules. No unused dependencies.
— Build + tests executed from prebuilt binaries: lib 11/11 pass, e2e 6/6 pass (including `e2e_full_session_through_the_app_stack` — a real WS session through bind/hello/token/dispatch/log), conformance 25/25 pass.
— boundaries.toml `[crates.control-server]` edges = exactly this manifest's internal deps (commands, control-protocol, product-config): no undeclared edge, no layering violation.

## 3. Findings
F-001 [S4]
Where: apps/control-server/Cargo.toml:1-3 vs docs/01-architecture.md:254
Code:
```
# apps/control-server — the optional loopback control surface (card
# 7.J1, issue #50; doc 01 §2.8 registry row: layer L6, "control-
# protocol, commands, engine, contracts"; doc 05 §5.4).
```
What is wrong: The quoted registry row (doc 01 §2.8, line 254: `| apps/control-server | app | L6 | control-protocol, commands, engine, contracts | 7 **[P]** | — |`) lists `engine` and `contracts` as this app's dependencies. The manifest declares neither; no crate named `contracts` exists anywhere in `crates/` (ls verified: 31 crates, none named contracts). The app's own comment (L12-16) and boundaries.toml (L624-627) correctly scope v1 to no-engine. Why it matters: doc 01 is the architecture registry — a reader planning against it believes the control server embeds the engine; the truth (introspection-only) requires reading the source. What correct looks like: registry row updated to "control-protocol, commands, product-config (engine join: desktop composition card)". Confidence: HIGH — Verification: read manifest + doc + ls crates/.
F-002 [S4]
Where: apps/control-server/Cargo.toml:40-45 vs product.toml:167 and .github/workflows/release.yml:193
Code:
```
[dependencies]
commands = { path = "../../crates/commands" }
control-protocol = { path = "../../crates/control-protocol" }
product-config = { path = "../../crates/product-config" }
serde_json = "1.0"
toml = "1"
```
What is wrong: Nothing in the manifest says the shipped product never turns this app on: `product.toml:167` gates `control-server = false` ("Phase 5+ [PROPOSAL]") and the release lane never builds the binary. The e2e test has to synthesize a `flipped_product()` (e2e.rs:231) just to exercise the real path — proof the production configuration never runs this code. Not a manifest defect per se (gating is the product model's job, doc 07 rule F2), but the manifest comments sell the app's contract without noting it ships dark. Confidence: HIGH — Verification: read product.toml + release.yml + e2e.rs.

## 4. Per-function table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comments | 1-23 | REAL | S4 | Honest about v1 scope (F-001's doc-01 quote is the drift); accurately describes the deferred engine join. |
| `[package]` | 25-30 | REAL | — | name/version/edition 2024/publish=false/description consistent with the repo (edition matches cli/desktop). |
| `[lib]` | 32-34 | REAL | — | `control_server` lib — used by main + tests. |
| `[[bin]]` | 36-38 | REAL | — | `lumen-control-server` — the entry the release lane does not build (F-002). |
| `[dependencies]` | 40-45 | REAL | — | All 5 used (rg census); externals match boundaries.toml's allowlist entry exactly. |
| `[lints] workspace = true` | 47-48 | REAL | — | Inherits deny-lists (unwrap/expect/unsafe) — clippy-clean per baseline cargo-clippy.txt. |

## 5. Test honesty
No tests in this file. Related test binaries executed: control_server lib (11 pass), e2e (6 pass), conformance (25 pass). The e2e's `flipped_product()` helper honestly mirrors the production-off gate rather than hiding it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "doc 01 §2.8 registry row: layer L6, 'control-protocol, commands, engine, contracts'" | Cargo.toml:2-3 quoting doc 01 | FALSE (the registry row, not the quote) | Manifest has no engine/contracts deps; no contracts crate exists (F-001). |
| "Externals: serde_json … + toml … both Lead-admitted via the crates that own them; the app adds no new external of its own" | boundaries.toml:628-630 | REAL | Manifest externals = {serde_json, toml} exactly; boundaries.toml allowlist confirms. |
| "`features.control-server` master gate" (comment) | Cargo.toml:11-12 | REAL | product.toml:167 declares the gate (currently false). |
| "v1 serves the catalog-introspection subset and answers everything else with a typed not-ready — an honest gap, never a silent no-op" | Cargo.toml:14-16 | REAL for commands / FALSE for events | Command side verified in dispatch.rs audit (2/65 + NotReady); the `subscribe` frame IS a silent no-op (dispatch.rs audit F-005). |

## 7. Dead / duplicated / unreachable code
- No unused dependencies (all 5 used). No duplicate dependency declarations.
- The binary target is unreachable from any shipped artifact (release lane + product gate both off) — the app is developer tooling.

## 8. Dependencies & flags
- Internal: commands (L1), control-protocol (L1), product-config — all path deps, matching boundaries.toml's downward-edge rule.
- External: serde_json 1.0, toml 1 — in the boundaries.toml external allowlist for this crate; no [features] section (product-model gating is runtime, not cargo).
- Inherits workspace lints; baseline clippy -D warnings CLEAN.

## 9. Open questions for the next agent
- Doc 01's "contracts" appears in the apps/cli and apps/desktop registry rows too — check whether ANY crate satisfies that name or whether the registry refers to `testkit`/contract-suite by a stale name (systemic doc-01 drift, one finding for the docs auditor).
- product.toml's `control-server = false # Phase 5+ [PROPOSAL]` vs the app being fully implemented (card 7.J1 ratified per the header): decide whether "Phase 5+" is still accurate — if the wave program's Phase 5 has passed, the comment and the gate state deserve a re-triage.
