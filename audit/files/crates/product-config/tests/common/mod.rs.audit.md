# AUDIT: crates/product-config/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines / functions | 74 / 9 (repo_root, root_schema, load_root_product, load_fixture, fixture_path, read_fixture, load_fixture_as, marker_line, for_key) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-support module (shared by tests/{fixtures_matrix,semantic,merge,rebrand_host,web_budgets,golden_s35}); not shipped by definition. The suites it serves run in `cargo test --workspace` — baseline green (product-config exit_code=0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read; sed quote verification; baseline cargo-test-workspace.txt product-config block; consumers verified by Grep (`use common::`) across tests/ |

## 1. One-paragraph verdict
Small, correct test-support module — every helper is used by at least one suite (verified by grep of `use common::` imports and per-helper call sites), the path math is right (`CARGO_MANIFEST_DIR` = crates/product-config, `join("../..")` = repo root — checked), and `marker_line`/`for_key` are the machinery behind the line-precise assertions audited in the five fixture audits. The single defect is rhetorical: the header claims "no `unwrap`/`expect`/`panic!` — workspace lints" and then the body uses `unreachable!()` as its panic-with-message — `unreachable!` is not in the workspace deny list (root Cargo.toml:59-66 denies unwrap_used/expect_used/panic/todo/unimplemented but not unreachable), so this is a lint-dodge that still panics, just under a name the linter ignores. The tree-wide count of this pattern is 4126 (baseline loc-patterns.txt, cited by AGENT-SPEC §6.4).

## 2. What actually works (verified)
- All 9 helpers have live consumers: `use common::{for_key, load_fixture, marker_line, read_fixture}` (fixtures_matrix.rs:8), `common::repo_root` (semantic.rs), merge/rebrand_host/web_budgets import subsets — the `#![allow(dead_code)]` at L9 is honest ("each suite uses a subset"), not a graveyard.
- Path math verified: repo_root() = crates/product-config + ../.. = workspace root; fixture_path joins `crates/product-config/tests/fixtures/<name>` — matches actual fixture locations (all 42 fixtures listed there).
- `root_schema()` loads the COMMITTED root product.schema.json — one schema source for all suites; no per-test schema forks.
- Baseline: product-config suites all pass with these helpers (fixtures_matrix 27, semantic 17, merge 15, rebrand_host 7, web_budgets 3, golden_s35 1 — exit_code=0).
- Direct execution of two dependent tests this session: semantic f1_unknown_feature_id_is_rejected, f1_f2_are_deferred_when_no_registry_is_supplied — both ok (they reach `read_fixture`/`root_schema` through this module's callers).

## 3. Findings
F-001 [S4]
Where: crates/product-config/tests/common/mod.rs:3-6 (header) vs 21-32 (root_schema) and 51-55 (read_fixture)
Code: (verbatim, L3-6 then L26-32)
```
//! Failure paths assert first and then diverge (no `unwrap`/`expect`/
//! `panic!` — workspace lints, doc 10 §3.1/§3.3); the divergence arms are
//! unreachable in practice because the assertion has already failed the
//! test.
```
```
        Err(err) => unreachable!("cannot read {}: {err}", path.display()),
    };
    match json.parse::<Schema>() {
        Ok(schema) => schema,
        Err(err) => unreachable!("root schema failed to load: {err}"),
```
What is wrong: the comment says the helpers avoid panics; `unreachable!` IS a panic (std::panic, same abort-the-test behavior as `expect`). The claim "the divergence arms are unreachable in practice because the assertion has already failed the test" does not describe these helpers at all — nothing asserts before `root_schema()` reads the file; a missing/corrupt product.schema.json panics here as the FIRST failure. The pattern exists because the workspace lint list omits `unreachable` — a lint-shaped hole, not a safety property.
Why it matters: the doc comment teaches contributors a false invariant ("unreachable arms are preceded by failed assertions"), and the pattern launders expect() through a macro the linter cannot see.
What correct looks like: either add `unreachable`-in-tests to the deny list with an explicit allow at these sites, or rewrite the comment: "these arms panic with a message when the environment is broken; that is the intended test failure."
Confidence: HIGH — Verification: read + root Cargo.toml lint list read

## 4. Per-function table (<100 LOC but included)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| repo_root | 12-15 | REAL | — | path math verified |
| root_schema | 21-32 | REAL | S4 | F-001 unreachable!-as-expect ×2 |
| load_root_product | 35-38 | REAL | — | load_file + root schema |
| load_fixture | 41-43 | REAL | — | used by all negative tests |
| fixture_path | 45-49 | REAL | — | path verified |
| read_fixture | 51-55 | REAL | S4 | F-001 unreachable! on I/O error |
| load_fixture_as | 59-61 | REAL | — | label override (doc 07 §3.5 repro) |
| marker_line | 64-68 | REAL | — | 1-based line of marker; powers line assertions |
| for_key | 71-74 | REAL | — | exact key_path filter; powers `one()` |

## 5. Test honesty
This module IS test machinery; its consumers' honesty audited per-suite (fixtures_matrix `one()` asserts exactly-one with full render on mismatch — would catch helper bugs like marker_line off-by-one, since d.line is compared to marker_line output).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no unwrap/expect/panic! — workspace lints" | L3-4 | FALSE-as-stated (unreachable! panics; see F-001) | read; root lint list lacks `unreachable` |
| "each suite uses a subset" (allow(dead_code)) | L9 | REAL | grep of use-common imports |

## 7. Dead / duplicated / unreachable code
None beyond the S4 pattern. All helpers consumed.

## 8. Dependencies & flags
None beyond the lib under test (product_config) + std. `#![allow(dead_code)]` is the only attribute.

## 9. Open questions for the next agent
- Should the workspace lint list close the `unreachable` hole (4126 tree-wide per baseline loc-patterns.txt)? Out of this file's scope; record for the hygiene wave.
