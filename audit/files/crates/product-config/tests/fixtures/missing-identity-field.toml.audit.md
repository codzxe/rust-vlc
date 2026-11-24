# AUDIT: crates/product-config/tests/fixtures/missing-identity-field.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 224 (negative fixture: product.toml fork, `identity.description_key` deleted) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only asset; consumed by fixtures_matrix.rs:74-81 (`missing_identity_field_points_at_the_table_header`). Not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff product.toml <fixture>` (exit 0: only the description_key line + stale-base deltas); baseline fixtures_matrix 27 passed; direct execution blocked (disk full) — baseline cited |

## 1. One-paragraph verdict
Mutation matches README.md:25 ("`identity.description_key` removed"). Verified by diff: the `description_key = "product.description"` line present in product.toml:32 is absent; sed shows the gap (comment block then straight to `[identity.ids]`):
```
version_code = 1
# Product description: a localization key, never literal text (MP §5).
# The prose lives in locales/en-US.toml under this key.

[identity.ids]
```
The consuming test asserts key `identity.description_key`, rule "required key is missing", found "absent", and that the diagnostic's line points at the `[identity]` table header — an honest, line-precise required-field test. Baseline green (27 passed). Same S4 fixture rot as siblings.

## 2. What actually works (verified)
- diff isolates the deletion (product.toml L32 gone, everything else = stale base).
- Test read at fixtures_matrix.rs:74-81: `assert_eq!(d.line, marker_line(&src, "[identity]"))` — pins the table-header pointing behavior.
- Baseline: fixtures_matrix `test result: ok. 27 passed; 0 failed`.

## 3. Findings
F-001 [S4] — inherited fork rot vs live product.toml (display_version 0.1.0-dev, version_code 1, min 480x320, streaming-http=false). Confidence: HIGH — Verification: ran diff.

## 4-9. (short audit)
Per-function table: N/A (data). Test honesty: exactly-one + rendered-diagnostics on mismatch. Doc claims: README.md:25 accurate. Dead code: none. Dependencies: none. Open questions: none.
