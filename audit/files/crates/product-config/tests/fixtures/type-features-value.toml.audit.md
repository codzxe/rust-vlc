# AUDIT: crates/product-config/tests/fixtures/type-features-value.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 226 / 0 (fixture data) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test data). Consumed by `tests/fixtures_matrix.rs::type_features_value_is_rejected` (fixtures_matrix.rs:225-231). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | baseline citation: audit/runs/baseline/cargo-test-workspace.txt:5101 `test type_features_value_is_rejected ... ok` (fresh rerun blocked by sandbox disk-full linker error — environment, UNVERIFIED live); diff vs root product.toml; sed verification |

## 1. One-paragraph verdict
Correct single-mutation type fixture: `core-playback = "yes"` (a string where the schema requires boolean), test pins key `features.core-playback`, rule "must be of type boolean", found `"\"yes\""`. Note the fixture keeps every other feature flag boolean and valid, so the features map itself is not the rejector — the type check on the single value is. Only defect: shared stale base (S4, README audit).

## 2. What actually works (verified)
— Baseline: `type_features_value_is_rejected ... ok` (cargo-test-workspace.txt:5101).
— Mutation (verbatim, L144-145):
```
core-playback = "yes"
```
— The chosen bad value is the classic TOML truthiness trap ("yes" is a string in TOML, not a boolean) — a realistic mistake, not a synthetic one.

## 3. Findings
F-001 [S4] Stale base snapshot (identical to the other big fixtures; see README.md audit). Confidence: HIGH — Verification: diff.

## 4-9. (short fixture audit)
Test honesty: exact assertions (key, rule substring, found value) — fails under type-check regression or diagnostic change. No dead code, no deps. Open questions: none.
