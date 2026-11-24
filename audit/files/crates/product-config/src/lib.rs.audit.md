# AUDIT: crates/product-config/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2025-06-14 |
| Lines / functions | 63 / module root: 11 modules + re-exports (crate total 3502 LOC) |
| File verdict | REAL (crate), with one dead re-export block |
| Reachable from shipped entry point? | NO for the release lane: lumen-cli and desktop do not depend on product-config. Consumed by apps/control-server (main.rs:135-154, config.rs — load_file/Schema/ProductConfig only) and the xtask/test lanes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | baseline citation (cargo-test-workspace.txt:5023-5187: product-config 70 integration tests + unit tests, exit 0); rg for runtime-helper consumers; grep Cargo.toml dependents; sed quote verification. |

## 1. One-paragraph verdict
A clean module root whose 11 declared modules all exist and are genuinely implemented (3502 LOC; the fixture matrix, golden, merge, rebrand, semantic and web-budget suites — 70 tests — pass per baseline). The pipeline doc (parse → structural → typed → semantic) matches the module layout. The one real problem: the `runtime` re-export block ships seven identity/rebrand helpers (about_facts, installer_identity, resolve_window_title, cli_version_line, …) that NO shipped binary calls — their only consumers are the rebrand acceptance tests, and the CLI's `--version`/usage lines hardcode "lumen" with an explicit [PROPOSAL] apology.

## 2. What actually works (verified)
— baseline audit/runs/baseline/cargo-test-workspace.txt:5023-5187: PACKAGE product-config exit_code=0 (fixtures_matrix 27, golden_s35 1, merge 15, rebrand_host 7, semantic 17, web_budgets 3, unit tests incl. suggest 5).

## 3. Findings
F-001 [S3]
Where: crates/product-config/src/lib.rs:58-61 vs apps/cli/src/help.rs:98-102 vs apps/ (no product-config dep)
Code (lib.rs:58-61):
```
pub use runtime::{
    AboutFacts, InstallerIdentity, about_facts, cli_usage_header, cli_version_line,
    installer_identity, log_line_prefix, log_startup_line, resolve_window_title,
};
```
vs apps/cli/src/help.rs:98-101:
```
pub fn version_text() -> String {
    // [PROPOSAL: product.toml identity (name/brand) joins with the
    // rebrand-propagation wiring; v0 reports the crate's own version.]
    format!("lumen {}\n", env!("CARGO_PKG_VERSION")) // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
```
What is wrong: the seven runtime identity helpers are consumed only by tests (rg across apps/ + crates/ excluding product-config itself: hits only in tests/rebrand/rebrand_acceptance.rs and product-config's own rebrand_host.rs). apps/desktop and apps/cli have no product-config dependency at all; control-server uses only the loader. This falsifies product.toml §2.1's "Every surface that shows a name reads from here at runtime" — the shipped CLI version line is a hardcoded literal with an apology comment. The helpers themselves are real and tested (rebrand_acceptance 7 PASS), but today they are a test-only rebrand simulation, not runtime wiring. What correct looks like: apps/cli help.rs and the desktop shell title path calling these helpers, or the docs marking them Phase-2 wiring. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (module root — module inventory instead)
| Module | LOC (wc) | Verdict | Notes |
|---|---|---|---|
| diagnostic | 237 | REAL | Diagnostic/Stage/render_all; unreachable! only in tests |
| lines | 240 | REAL | LineIndex for diagnostics |
| load | 202 | REAL | load_file/str + partial loaders |
| merge | 487 | REAL | Layered precedence; 15 merge tests PASS |
| model | 260 | REAL | ProductConfig typed model |
| partial | 165 | REAL | PartialProduct |
| patterns | 324 | REAL | Schema pattern matchers (unreachable! in test only) |
| runtime | 213 | REAL | S3: identity helpers, test-only consumers |
| schema | 639 | REAL | Data-driven schema validation |
| semantic | 565 | REAL | Cross-field/F1/F2/asset magic checks; 17 tests PASS |
| suggest | 107 | REAL | See suggest.rs audit |

## 5. Test honesty
Covered by the six integration suites (baseline all PASS). lib.rs itself has no tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Typed, validated model … with actionable diagnostics" | lib.rs:3-4 | REAL | 70 baseline tests |
| "no unsafe/unwrap/expect/panic (workspace lints)" | lib.rs:39 | REAL for src (all panic-family hits are #[cfg(test)]) | rg: 7 hits, all in test modules |
| (inherited) "Every surface that shows a name reads from here at runtime" | product.toml §2.1 | FALSE today | F-001: helpers test-only; CLI hardcodes "lumen" |

## 7. Dead / duplicated / unreachable code
The runtime identity re-export block (F-001) — real code, no shipped consumer.

## 8. Dependencies & flags
toml, thiserror, serde(+derive) per Cargo.toml comments; no feature gates; no unsafe.

## 9. Open questions for the next agent
- Rebrand wiring card: who connects apps/cli + apps/desktop to product_config::runtime (the [PROPOSAL] in help.rs:99-100)?
- Is apps/control-server in any release lane? It is the only production consumer of this crate.
