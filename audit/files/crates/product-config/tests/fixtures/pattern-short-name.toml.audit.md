# AUDIT: crates/product-config/tests/fixtures/pattern-short-name.toml
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines | 204 (full product.toml fork) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/fixtures_matrix.rs:237 (`pattern_short_name_is_rejected`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff product.toml vs fixture (mutation: `short_name = "Lumen Player"` at :22); sed test body verification |

## 1. Verdict
Honest fixture. One intentional mutation on a product.toml fork:
`short_name = "Lumen Player"` (product.toml:22 has `"Lumen"`). The space
violates the short_name pattern (schema requires a compact slug). The test
(fixtures_matrix.rs:236-242) asserts the fixture is REJECTED (`rejected()`
unreachable!s on success), exactly one `identity.short_name` diagnostic,
rule containing "must match the pattern", found `"Lumen Player"`. Mutation
reasoning: if the schema walker stopped enforcing patterns, `one()` gets 0
hits → fails. Note: this is STRUCTURAL-stage coverage — and the xtask
config-validator (the tool that is supposed to enforce this in CI) admits
"pattern constraints not enforced by this v0" (config_validator.rs:117);
this crate-side test is currently the only automated enforcement of the
pattern rules. Passed in baseline.

## 2. Findings
F-001 [S4]
What is wrong: stale-fork drift (display_version "0.1.0-dev", min sizes
480/320, no [web.size_budgets]) — same benign family as the siblings.
Confidence: HIGH — Verification: diff.
