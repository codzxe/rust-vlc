# AUDIT: crates/product-config/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 24 / config keys: package, lints, dependencies×4 |
| File verdict | UNREACHABLE (real, tested typed model — but the only real dependent is the unshipped control-server app; the shipped gates parse product.toml with xtask's own toml_lite instead) |
| Reachable from shipped entry point? | NO. Proof: workspace-wide `[dependencies]` scan → sole real dependent is apps/control-server/Cargo.toml (itself unshipped, see that audit); plugin-api and settings-schema mention product-config in comments only (plugin-api/Cargo.toml:6, settings-schema/Cargo.toml:24 — no dep edge); apps/cli, apps/desktop, desktop-tauri: no dep; the shipped product.toml gate (xtask config-validator) parses with its own `toml_lite` (tools/xtask/src/cmd/config_validator.rs:29) — not this crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | per-manifest `[dependencies]` scan loop (one REAL-DEP: control-server); `rg -n 'product_config'` in plugin-api/src, settings-schema/src, apps/cli/src (0 code hits — comment-only mentions); boundaries.toml:96-101 diff (exact); docs/dependencies/preaudit-foundation.md:24-28 read; lock greps (serde 1.0.229 ×1, thiserror 2.0.21 ×1, serde_json 1.0.154 ×1, toml 1.1.8+spec-1.1.0 ×1); dep-usage rg (serde 2, serde_json 3, toml 1, thiserror 1 src files) |

## 1. One-paragraph verdict
The manifest itself is coherent — four externals, all matching boundaries.toml:96-101 and all resolving in the lock to exactly the versions the pre-audit table blessed — and all four deps are genuinely used in src. The defect is structural: the crate billed as THE typed model of product.toml (doc 01 §2.3) is linked into exactly one consumer, the unshipped control-server app. The shipped `lumen` binary never touches it; the CI gate that actually validates product.toml (xtask config-validator, baseline #10) re-implements TOML parsing (toml_lite) and its own JSON-Schema-subset walk rather than using this model. Two parallel product.toml interpretations exist, and the typed one is dead in every shipped artifact.

## 2. What actually works (verified)
— Boundaries match: external_allowlist [serde, serde_json, toml, thiserror] == manifest deps; internal_deps [] == no path deps.
— Pre-audit honesty: docs/dependencies/preaudit-foundation.md:24-28 lists all four (serde 1.0.229, serde_json 1.0.154, toml 1.1.8+spec-1.1.0, thiserror 2.0.21, all "**pass**"); Cargo.lock resolves each to exactly those versions, one entry each.
— All deps used in src: serde (derive) 2 files, serde_json 3, toml 1, thiserror 1.
— The model is exercised: apps/control-server/tests/e2e.rs builds `product_config::Schema` from the committed product.schema.json and validates a flag-flipped temp copy (part of baseline #3's 1681 passes).

## 3. Findings
### F-001 [S3]
Where: crates/product-config/Cargo.toml:1-8 (crate identity) vs workspace dependency graph
Code:
```
# crates/product-config — the config-driven product model (doc 01 §2.3).
#
# L1 contracts & config crate (registry name: `product-config`).
# Responsibility: typed, validated model of `product.toml` (against
# `product.schema.json`), actionable diagnostics, and layered resolution
# (doc 07). No internal deps (doc 01 §2.3 (c))
```
What is wrong: The crate that owns "typed, validated model of product.toml" has exactly one real dependent — apps/control-server — which ships nowhere (control-server audit F-001: absent from all packaging payloads, product gate false). The shipped `lumen` CLI and the tauri shell never link it. Meanwhile the gate that does validate product.toml in CI (xtask config-validator, baseline #10: "product.toml vs product.schema.json") deliberately re-implements parsing (`toml_lite`, config_validator.rs:29 `use crate::cmd::toml_lite::{self, Value as TValue}`) because "xtask is std-only by C5".
Why it matters: Product-identity validation is split across two independent implementations — a typed model that is dead code in every shipped artifact, and a bespoke TOML-subset+schema walker that is the only validator that ever runs in the gate. They can drift: a product.toml shape toml_lite accepts but the typed model rejects (or vice versa) would surface only in control-server's e2e, which nobody ships. The registry's "single source of truth" story (doc 01 §2.3) is contradicted by the graph.
What correct looks like: Either the CLI/tauri hosts consume product-config for their product-model needs (making the typed model the real consumer), or the crate is folded into control-server as a private module until a second consumer exists, or config-validator's doc explicitly subordinates toml_lite to the typed model with a parity test between the two.
Confidence: HIGH — Verification: read (dependency-graph scan loop; config_validator.rs:1-29; preaudit table).

### F-002 [S4]
Where: crates/product-config/Cargo.toml:21-24
Code:
```
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "1"
thiserror = "2"
```
What is wrong: Loose semver specs while the cited pre-audit rows pin exact versions (preaudit-foundation.md:24-28). A `cargo update` would silently move to versions no pre-audit ever blessed, defeating the preaudit-foundation regime the header leans on.
Why it matters: The preaudit table's "[VERIFY-P1 … confirm at pin time]" for toml's license explicitly defers verification to pin time — but this manifest never pins, so "pin time" never comes.
What correct looks like: `serde = "=1.0.229"`, `serde_json = "=1.0.154"`, `toml = "=1.1.8+spec-1.1.0"`, `thiserror = "=2.0.21"` (current lock resolutions) — the same house style streaming uses for ureq.
Confidence: HIGH — Verification: read (preaudit rows + lock).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` | 10-15 | REAL | — | description accurate for the crate's content; reachability failure is a graph property (F-001), not a description lie. |
| `[lints] workspace = true` | 17-18 | REAL | — | Inherited (file-1 loop-grep). |
| `serde` (+derive) | 21 | REAL | S4 (F-002) | Used (2 src files); lock 1.0.229 == preaudit row. |
| `serde_json` | 22 | REAL | S4 (F-002) | Used (3 src files); lock 1.0.154 == preaudit row. |
| `toml` | 23 | REAL | S4 (F-002) | Used (1 src file); lock 1.1.8+spec-1.1.0 == preaudit row (with its VERIFY-P1 license caveat). |
| `thiserror` | 24 | REAL | S4 (F-002) | Used (1 src file); lock 2.0.21 == preaudit row. |

## 5. Test honesty
No tests in manifest. The crate's model is exercised via control-server's e2e (schema load + validation of a temp product.toml); baseline #3 includes it. Per-test honesty: product-config source agents.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "external deps are all pre-audit pass rows in docs/dependencies/preaudit-foundation.md: serde 1.x, serde_json 1.x, toml 1.x, thiserror 2.x" | Cargo.toml:6-8 | REAL | Table rows :24-28 all "pass"; lock matches each version exactly. |
| "No internal deps (doc 01 §2.3 (c))" | Cargo.toml:6 | REAL | [dependencies] has no path deps; boundaries.toml:98 internal_deps = []. |
| Registry: product-config is the product model layer (doc 01 §2.3) | Cargo.toml:1-3 citing doc 01 | SHALLOW (as shipped reality) | Sole real consumer is the unshipped control-server; the shipped gate uses toml_lite instead (F-001). |

## 7. Dead / duplicated / unreachable code
The whole crate is unreachable from shipped entry points (F-001) while being fully compiled in every workspace build (baseline #1) and tested (baseline #3). Duplication: product.toml understanding exists twice (typed model here; toml_lite + schema walk in xtask) — cross-ref tools/xtask agents.

## 8. Dependencies & flags
No `[features]`, no dev-deps, no build-deps, no internal deps. L1 crate with four externals — the heaviest external footprint of the audited manifests, all preaudited.

## 9. Open questions for the next agent
- If product-config is the intended engine of the rebrand/rename story (product.toml brand identity), why does the shipped CLI read its identity from hard-coded literals that lint-literals/check-identity police (baseline #13/#14 failure at apps/cli/src/host.rs:1799) instead of this model? Trace whether lumen CLI embeds product identity from product.toml at all — if not, rebrand tooling and the typed model are both bypassed by the shipped binary.
- config_validator.rs:4 admits "until now it did not exist (CI covered product.toml only via the …)" — what covered it before, and did anything validate product.toml between the crate's creation and 8.F1? History question for the worklog agents.
