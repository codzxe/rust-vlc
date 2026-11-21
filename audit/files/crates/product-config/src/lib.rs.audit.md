# AUDIT: crates/product-config/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 (re-spawned instance; files 1–3 by the previous instance of the same Task ID) |
| Date | 2026-11-21 |
| Lines / functions | 63 / 0 functions, 0 tests (11 `pub mod` decls + 11 `pub use` groups re-exporting 24 names) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO. `rg "product-config" --glob Cargo.toml` over the repo → exactly one dependent: apps/control-server/Cargo.toml:43. The packaging gate stages only the trio `lumen-cli` / `desktop` / `lumen-desktop-tauri` (scripts/packaging/gate.sh:63–93 BIN_CLI/BIN_DESKTOP/BIN_TAURI; `rg control-server scripts/packaging/gate.sh` → no hits), and control-server's feature is `false` in the shipped product.toml:167 (`control-server = false  # Phase 5+ [PROPOSAL]`) — its load runs pre-gate (apps/control-server/src/main.rs:135–154) but that binary is not in the shipped payload. YES from workspace tests: tests/rebrand/rebrand_acceptance.rs:41–43 imports through this lib.rs surface (7/7 ok, orchrun log:4721–4732). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "product-config" --glob Cargo.toml` (1 dependent: control-server:43) · `rg -n "product_config::" apps/ crates/ tools/ tests/` (consumer census: control-server src/tests, crate tests, tests/rebrand — nothing in tools/) · `rg -n "load_partial_file" crates/product-config/ apps/ tests/ tools/` (only def load.rs:163 + re-export lib.rs:54 — ZERO callers, not even tests) · `rg` for `Severity|RequiredMode|SchemaError|LayerSource|resolve_layers|PartialProduct|render_all|load_partial_str|\bLayer\b|\blayer\b` over apps/, tests/, tools/, crates/product-config/tests/ (re-export liveness census) · `rg -n "\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|unsafe " crates/product-config/src/` (zero hits — lib.rs:39 hygiene claim) · `rg -n -A20 '^\[workspace.lints' Cargo.toml` (deny lints, :53–66) · `sed -n 433,460p docs/01-architecture.md` (§2.3 product-config (d) at :452) · `sed -n 1,60p tools/xtask/src/cmd/config_validator.rs` (card 8.F1, std-only, no product-config dep) · `sed -n 1,99p crates/product-config/src/load.rs` (parse diagnostics carry `key_path: "<document>"`) · `sed -n 60,92p scripts/packaging/gate.sh` (shipped trio) · baseline: build #1 exit 0; clippy #5/#6 clean; orchrun-test-workspace-nofailfast.log.txt:4613/4658 (41/41 unit), 4693 (golden), 4700–4719 (merge 15/15), 4721–4732 (rebrand 7/7), 4734 (semantic), 4757 (web_budgets); xtask-config-validator run #10 exit 1. |

## 1. One-paragraph verdict
The crate root is mechanically clean — every declared module exists, every one of the 24 re-exported names resolves (baseline build #1 compiles it; clippy clean with `unwrap/expect/panic/todo/unimplemented` denied at workspace level, and a direct grep confirms zero hits), and the sibling files behind each `pub mod` were audited REAL-code by W11-A01…A09. But this is the front door of a library that no shipped binary links: the sole production dependent is `apps/control-server`, which is feature-gated off in the shipped product.toml and absent from the packaging payload, and it consumes only 4 of the 24 re-exported names. The crate doc carries two misleading claims — the four-stage pipeline's Semantic stage is wired to nothing (the "card 1.E2" xtask wiring was superseded by a std-only parallel validator, card 8.F1), and "every failure carries … fully-qualified key path" overstates what parse-stage diagnostics carry. The heavy unconsumed-surface findings (typed model bypassed, layering API caller-less, runtime resolvers test-only) are already recorded at S2 on their implementing files by the other agents; they are cross-referenced here, not re-issued.

## 2. What actually works (verified)
— Crate compiles + lints clean: baseline #1 `cargo build --release --locked --workspace` exit 0; #5/#6 clippy `-D warnings` exit 0, with `[workspace.lints]` denying `unsafe_code`, `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` (Cargo.toml:53–66) — so lib.rs:39's "no `unsafe`/`unwrap`/`expect`/`panic`" claim is enforced, and `rg` over `crates/product-config/src/` for those constructs returns zero hits.
— The re-export surface is real and compile-pinned by tests: tests/rebrand/rebrand_acceptance.rs:41–43 imports `ProductConfig, Schema, about_facts, cli_usage_header, cli_version_line, installer_identity, load_str, log_line_prefix, log_startup_line, render_all, resolve_window_title` through this exact lib.rs surface and ran 7/7 ok in the baseline no-fail-fast run (orchrun log:4721–4732); `product_config` unittests 41/41 ok (log:4613, 4658); merge 15/15 (4700–4719); golden_s35/semantic/web_budgets/fixtures_matrix all ok (4693–4761).
— Runtime consumer exists (unshipped): apps/control-server/src/main.rs:145,154 + config.rs:26,55,240 use `product_config::{Schema, load_file, ProductConfig, Diagnostic}` through this root.
— lib.rs:5 "layer L1, no internal deps": verified — crates/product-config/Cargo.toml:20–24 declares only serde, serde_json, toml, thiserror (all external); no workspace crate depends inward.

## 3. Findings

### F-001 [S3]
Where: crates/product-config/src/lib.rs:14–19
Code:
```rust
//! 4. **Semantic** — [`semantic::validate_semantics`]: cross-field rules,
//!    injected enums from directory listings, catalog-key existence,
//!    feature-registry intersection (F1/F2), brand-asset existence + magic
//!    bytes. Inputs that live in files owned by other cards are explicit
//!    ([`semantic::SemanticInputs`]); the `cargo xtask config-validator`
//!    wiring is card 1.E2.
```
What is wrong: The crate-root pipeline doc tells the reader that stage 4 (semantic: F1/F2 feature-registry intersection, brand-asset existence + magic bytes) is wired into `cargo xtask config-validator` via "card 1.E2". That wiring does not exist. The shipped xtask validator is card 8.F1 (its own header: "issue #20, card 8.F1 … until now it did not exist", tools/xtask/src/cmd/config_validator.rs:1–5), is std-only via `toml_lite`, has zero product-config dependency (`rg product_config:: tools/` → no hits), and explicitly punts `pattern` constraints, F1/F2, and magic-byte checks ("Honest v0 limits", config_validator.rs:17–27). `validate_semantics` itself has zero production callers (rg repo-wide: definition semantic.rs:141, re-export lib.rs:63, and the crate's own tests/semantic.rs:22,289 — merge.rs:217 calls only `cross_field_checks`, itself on the test-only `resolve_layers` path).
Why it matters: This is the crate's front-door description of the validation architecture, and it teaches the wrong one. A reader concludes the CI gate (`cargo xtask config-validator`) exercises this crate's semantic stage; in reality baseline run #10's 4 violations came from the weaker parallel tool, and the rules engine that would catch "config declares features the engine doesn't have" (the Wave-8 `audio.set-eq` disease) runs in no shipped or gate path. Same false claim in stronger present tense at semantic.rs:7–10 (recorded S2 there by W11-A04 — cross-ref, not double-counted).
What correct looks like: lib.rs states the truth: xtask config-validator (8.F1) is an independent std-only subset; the semantic stage currently runs only in the crate's committed test suite until a host wires it. Or wire the crate into xtask for real.
Confidence: HIGH — Verification: read (config_validator.rs:1–60, semantic.rs callers, rg census) + baseline #10 output; cross-ref audit/files/crates/product-config/src/semantic.rs.audit.md F-001.

### F-002 [S3]
Where: crates/product-config/src/lib.rs:53–63
Code:
```rust
pub use diagnostic::{Diagnostic, Severity, Stage, render_all};
pub use load::{load_file, load_partial_file, load_partial_str, load_str};
pub use merge::{Layer, LayerSource, layer, resolve_layers};
pub use model::ProductConfig;
pub use partial::PartialProduct;
pub use runtime::{
    AboutFacts, InstallerIdentity, about_facts, cli_usage_header, cli_version_line,
    installer_identity, log_line_prefix, log_startup_line, resolve_window_title,
};
pub use schema::{RequiredMode, Schema, SchemaError};
pub use semantic::{FeatureRegistry, SemanticInputs, validate_semantics};
```
What is wrong: The declared public API is 24 names; the consumer census (rg over apps/, tools/, tests/, crates/product-config/tests/): the only production consumer (control-server, itself unshipped) uses 4 (`ProductConfig`, `Schema`, `Diagnostic`, `load_file`). `load_partial_file` (line 54) has ZERO callers anywhere in the repo — not even a test (only the definition load.rs:163 and this re-export; the crate's tests use `load_partial_str`: tests/web_budgets.rs:61, tests/merge.rs:23,77,96). `Severity`, `RequiredMode`, `SchemaError` have no users outside the crate; `Severity::Warning` is additionally never constructed anywhere (diagnostic.rs audit F-002). The 7 runtime resolvers (lines 58–61) are used only by tests/rebrand (runtime.rs audit F-001, S2); the layering names (`Layer`, `LayerSource`, `layer`, `resolve_layers`, `PartialProduct`, `load_partial_str`) only by the crate's own tests (merge.rs audit F-002, S2). This also corrects the load.rs audit's reachability row, which lumps `load_partial_file` with its `_str` sibling as "tests only" — the file variant is dead outright.
Why it matters: The crate root is the declared-vs-consumed surface this wave interrogates: 20 of 24 exported names have no production consumer and one is dead even under test. Dead API surface misleads (readers assume a loader for user-layer files exists and is exercised) and rots silently.
What correct looks like: Delete `load_partial_file` (or test it); mark the Phase-2+ layering and runtime re-exports with the same "no host yet" honesty the runtime.rs doc already carries; consider `#[doc(hidden)]` or a consumer-census comment per group.
Confidence: HIGH — Verification: rg census (Commands run); every usage site listed above read.

### F-003 [S4]
Where: crates/product-config/src/lib.rs:30–31 (claim); crates/product-config/src/load.rs:40, 61, 87 (evidence)
Code:
```rust
//! Every failure carries file + line + fully-qualified key path + the rule
//! as a constraint + expected vs found + a fix hint; near-miss keys get a
//! "did you mean" hint. Invalid critical config is never silently replaced
//! (MP §5).
```
```rust
        key_path: "<document>".to_string(),
```
What is wrong: The universal quantifier overclaims: parse-stage failures (TOML syntax, load.rs:40), TOML→JSON conversion failures (load.rs:61), and typed-deserialize residual failures (load.rs:87) carry the sentinel `key_path: "<document>"`, not a fully-qualified key path — no key exists for those failures, but the doc asserts every failure carries one. File, line, rule, expected/found, and hint are genuinely present in all cases, and the key-addressable (structural/semantic) diagnostics do carry real key paths.
Why it matters: Doc precision on the crate's central contract; the golden test (golden_s35.rs) pins the real format, so only the prose overstates.
What correct looks like: "Every failure carries file + line + rule + expected vs found + a fix hint; failures that are key-addressable additionally carry the fully-qualified key path."
Confidence: HIGH — Verification: read load.rs:28–99 (all three diagnostic constructors).

## 4. Per-item table (no functions in this file; every module declaration and re-export gets a row)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod diagnostic` | 41 | REAL (module, A01) | — | File exists; audited REAL by W11-A01. |
| `pub mod lines` | 42 | REAL (module, A03) | — | Audited REAL by W11-A03. |
| `pub mod load` | 43 | REAL (module, A03) | — | Base loader real; partial half caller-less (A03 S2). |
| `pub mod merge` | 44 | REAL/UNREACHABLE (A09) | — | Whole API test-only (A09 F-002, S2). |
| `pub mod model` | 45 | UNREACHABLE (A05 prev.) | — | Typed model bypassed by all shipped paths (prev. instance F-001, S2). |
| `pub mod partial` | 46 | UNREACHABLE (A02) | — | Zero consumers outside crate tests. |
| `pub mod patterns` | 47 | UNREACHABLE (A06) | — | Real matchers; shipped trio never reaches them. |
| `pub mod runtime` | 48 | UNREACHABLE (A08) | — | 7 resolvers, zero production callers (A08 F-001, S2). |
| `pub mod schema` | 49 | REAL code/UNREACHABLE (A02) | — | Genuine engine; no shipped edge. |
| `pub mod semantic` | 50 | UNREACHABLE (A04) | — | F-001 cross-ref; test-only. |
| `pub mod suggest` | 51 | REAL (A01) | — | Wired into schema.rs unknown-key path. |
| `pub use diagnostic::{Diagnostic, Severity, Stage, render_all}` | 53 | PARTLY DEAD | S3 (F-002) | `Diagnostic`: control-server; `Stage`/`render_all`: tests only; `Severity`: no external user, `Warning` never constructed. |
| `pub use load::{load_file, load_partial_file, load_partial_str, load_str}` | 54 | PARTLY DEAD | S3 (F-002) | `load_file`: control-server only; `load_str`: tests/rebrand + control-server tests; `load_partial_str`: crate tests; `load_partial_file`: ZERO callers anywhere. |
| `pub use merge::{Layer, LayerSource, layer, resolve_layers}` | 55 | TEST-ONLY | — | crates/product-config/tests/merge.rs only (A09). |
| `pub use model::ProductConfig` | 56 | USED (unshipped host) | — | control-server main.rs:135, config.rs:26 + tests/rebrand:42. |
| `pub use partial::PartialProduct` | 57 | TEST-ONLY | — | tests/merge.rs only. |
| `pub use runtime::{7 names}` | 58–61 | TEST-ONLY | — | tests/rebrand/rebrand_acceptance.rs:42–43 only (A08 F-001). |
| `pub use schema::{RequiredMode, Schema, SchemaError}` | 62 | PARTLY DEAD | S3 (F-002) | `Schema`: control-server + tests; `RequiredMode`/`SchemaError`: zero external users. |
| `pub use semantic::{FeatureRegistry, SemanticInputs, validate_semantics}` | 63 | TEST-ONLY | — | tests/semantic.rs only; no host, no xtask wiring (F-001). |

## 5. Test honesty (no tests in this file)
| Test suite exercising this file's surface | Would it fail if this file were broken? | Reason |
|---|---|---|
| tests/rebrand/rebrand_acceptance.rs (7/7 ok, orchrun:4721–4732) | YES (compile-level) | Imports 11 names through lib.rs's re-exports; deleting or mis-routing any one breaks compilation of the suite — the surface is compile-pinned by tests, though no runtime production path pins it. |
| product_config unittests 41/41 (orchrun:4613,4658) + merge 15/15 (4700–4719) + semantic/golden/web_budgets (4693–4761) | YES (indirectly) | They exercise the modules this file declares; module mis-declaration breaks the build. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Typed, validated model of `product.toml` against `product.schema.json`" | lib.rs:3 | REAL | Stages 1–3 executed on the committed files in baseline (control-server unit tests + rebrand 7/7, orchrun:4613–4761); schema.rs/load.rs audits (A02/A03). |
| Pipeline stage 4 "Semantic … the `cargo xtask config-validator` wiring is card 1.E2" | lib.rs:14–19 | FALSE (as a description of today's tree) | F-001: xtask validator is card 8.F1, std-only, no product-config dep (config_validator.rs:1–27); `validate_semantics` callers = tests only. |
| Structural stage reports "every violation" | lib.rs:12 | REAL | `schema.validate` returns an aggregate Vec; `load_str` returns it whole (load.rs:78–81); fixtures_matrix asserts multi-violation output (A02/A03 audits). |
| "v1 implements product defaults + user: `load_str` … `load_partial_str` … `resolve_layers`/`layer`" | lib.rs:21–26 | REAL-UNPROVEN / UNREACHABLE in shipped product | API real + 15/15 merge tests ok (orchrun:4700–4719), but zero shipped callers (A09 F-002, S2); `config.set-setting` writes an in-memory BTreeMap that dies with the process. |
| "Every failure carries file + line + fully-qualified key path + rule + expected vs found + fix hint; did-you-mean" | lib.rs:30–32 | REAL (with F-003 overbreadth) | diagnostic.rs golden test pins the format (A01); "<document>" sentinel for parse failures (F-003); did-you-mean wired at schema.rs:362 (A01 suggest.rs audit). |
| "Invalid critical config is never silently replaced (MP §5)" | lib.rs:32–33 | REAL in-crate / MOOT in product | `load_str`/`load_partial_str` return `Err(Vec<Diagnostic>)`, no fallback path (load.rs:57–99); but no shipped surface loads product.toml at all, so nothing enforces it where users live. |
| "No playback logic, no silent fallback, no hardcoded user-visible strings (doc 01 §2.3 (d))" | lib.rs:36–38 | REAL | Doc 01 §2.3 product-config (d) matches (docs/01-architecture.md:452); product strings all derive from config (runtime.rs, rebrand tests); diagnostic prose is operator-facing, outside the rebrand surface. |
| "no `unsafe`/`unwrap`/`expect`/`panic` (workspace lints)" | lib.rs:39 | REAL | rg over src/: zero hits; workspace lints deny all five (Cargo.toml:53–66); clippy baselines #5/#6 clean. |
| "layer L1, no internal deps" | lib.rs:5 | REAL | crates/product-config/Cargo.toml:20–24: serde/serde_json/toml/thiserror only. |

## 7. Dead / duplicated / unreachable code
- Fully dead: `load_partial_file` (re-exported lib.rs:54, defined load.rs:163, zero callers incl. tests) — F-002.
- No-external-user re-exports: `Severity`, `RequiredMode`, `SchemaError` — F-002.
- Test-only re-export groups: merge (55), partial (57), runtime (58–61), semantic (63), plus `load_partial_str`/`Stage`/`render_all`.
- Whole-crate unreachable from the shipped trio (packaging gate payload, gate.sh:63–93); sole dependent control-server is feature-off (product.toml:167) and unshipped.
- Duplication around this file (cross-refs, owned by other audits): THREE parallel product.toml validators now exist — this crate's schema engine, xtask's std-only toml_lite walker (8.F1), and desktop-tauri's `parse_window_table` (app.rs:243). lib.rs's doc acknowledges none of them; each has different coverage (xtask skips patterns/F1/F2/magic-bytes; tauri's subset parser accepts what patterns.rs rejects).

## 8. Dependencies & flags
- crates/product-config/Cargo.toml: `[lints] workspace = true`; `[dependencies]` serde 1 (derive), serde_json 1, toml 1, thiserror 2 — all external, matching the preaudit-foundation pass rows cited in its header; no `[features]`, no dev-dependencies beyond the committed test fixtures; edition 2024; `publish = false`.
- No feature flags gate any of this crate's code; nothing here is cfg'd out in shipped builds — it is simply never linked by them.

## 9. Open questions for the next agent
- Wave gate: the "card 1.E2 wiring" claim exists in BOTH lib.rs:18–19 and semantic.rs:7–10 — one fix decision (wire xtask to the crate, or fix both docs) covers both files.
- Who owns dedup of the three product.toml validators (crate engine / xtask toml_lite / tauri parse_window_table)? Each audit found the others; no single owner is recorded anywhere.
- Correction for W11-A03's load.rs audit reachability row: `load_partial_file` is not "tests only" — it has zero callers at all (this audit, F-002).
- If control-server ever ships, note that its load runs BEFORE the feature gate (main.rs:135–154), so the whole crate would become shipped-reachable through a binary whose feature is off by default — decide whether that is the intended delivery vehicle.
