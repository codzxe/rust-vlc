# AUDIT: crates/product-config/tests/fixtures/type-window-width.toml
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines | 204 (full product.toml fork) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/fixtures_matrix.rs:217 (`type_window_width_is_rejected`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff product.toml vs fixture (mutation: `width = "wide"` at :64-65); sed test body verification |

## 1. Verdict
Honest fixture. One intentional mutation on a product.toml fork:
`width = "wide"` (product.toml:64 has `width = 1280`). A string where the
schema demands integer. The test (fixtures_matrix.rs:215-221) asserts
rejection, exactly one `window.width` diagnostic, rule containing "must be
of type integer", found `"wide"`. Mutation reasoning: if the schema walker's
type check broke, `one()` gets 0 hits → fails. Passed in baseline
(product-config exit 0). Together with range-window-width.toml and
pattern-short-name.toml this covers the type/range/pattern trio for the same
key's neighborhood.

## 2. Findings
F-001 [S4]
What is wrong: stale-fork drift (display_version "0.1.0-dev", min sizes
480/320, no [web.size_budgets]) — benign; the mutation key `width = "wide"`
short-circuits before cross-field geometry checks apply (typed value never
reaches the semantic stage).
Confidence: HIGH — Verification: diff.
