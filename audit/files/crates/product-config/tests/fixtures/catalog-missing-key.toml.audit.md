# AUDIT: crates/product-config/tests/fixtures/catalog-missing-key.toml
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines | 203 (full product.toml fork) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/semantic.rs:179 (`title_idle_key_must_exist_in_the_reference_catalog`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff product.toml vs fixture (mutation: `title_idle_key = "window.title.gone"` at :80); sed test body verification |

## 1. Verdict
Honest fixture. One intentional mutation on top of a product.toml fork:
`title_idle_key = "window.title.gone"` (product.toml:84 has
`window.title.idle`). The consuming test (semantic.rs:178-193) supplies a
reference catalog containing `product.description` and `window.title.idle`,
asserts exactly ONE `window.title_idle_key` diagnostic whose `found` is
`"window.title.gone"`, AND asserts zero diagnostics for
`identity.description_key` (positive control — the in-catalog key passes).
The helper `semantic_diagnostics` (semantic.rs:12-22) first asserts the
fixture PASSES structural validation, so the mutation is isolated to the
semantic layer. Mutation reasoning: if `catalog_checks` stopped checking,
`one()` gets 0 hits → test fails. Passed in baseline (product-config exit 0).

## 2. Findings
F-001 [S4]
What is wrong: the fork is a stale product.toml snapshot (display_version
"0.1.0-dev" vs current "0.1.3", min sizes 480/320 vs 940/600, no
[web.size_budgets]). Benign — stale values remain schema-valid and satisfy
cross-field rules (480 < 1280), so no unintended extra diagnostics — but the
fixture does not track the live file.
Confidence: HIGH — Verification: diff.
