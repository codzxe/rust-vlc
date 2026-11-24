# AUDIT: crates/product-config/tests/fixtures/pattern-title-template.toml
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines | 226 (full product.toml copy, one mutation) |
| File verdict | REAL |
| Reachable | Test fixture — consumed by crates/product-config/tests/fixtures_matrix.rs:281-294 `pattern_title_template_unknown_variable_is_rejected` (passes, baseline: `test pattern_title_template_unknown_variable_is_rejected ... ok`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Single-mutation fixture: `title_template = "{unknown_var} — {product_name}"` (line 78) against the schema pattern `^(\{product_name\}|\{media_title\}|[^{}]|\{\{)*$` (product.schema.json:122) — an unknown template variable must be a structural error per doc 07 §2.2. The test asserts the diagnostic for `window.title_template`, rule "must match the pattern", the x-lumen-hint mentioning `{product_name}`, and the exact source line of the mutation:
```
title_template = "{unknown_var} — {product_name}"
```
The em-dash separator and the valid `{product_name}` variable are retained in the mutated value, so the matcher must reject specifically because of `{unknown_var}`, not because of braces or the dash — a precise probe of `is_title_template` (patterns.rs:28, 81+). Would fail if the matcher accepted unknown variables or rejected valid ones.

F-001 [S4] Stale base (0.1.0-dev / version_code 1 / 480x320 / no `[web.size_budgets]`) — same class as sibling fixtures; structural gate in the test helper (`rejected`/`read_fixture` path via fixtures_matrix `load`) keeps the drift loud. Confidence: HIGH — Verification: read + diff.

## Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_matrix.rs:281 pattern_title_template_unknown_variable_is_rejected | YES | pins key, rule, hint content, and exact line via `marker_line(&src, "title_template = \"{unknown_var}")` |
