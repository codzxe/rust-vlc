# AUDIT: crates/product-config/tests/fixtures/missing-defaults-key.toml (fixture)
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines | 239 (product.toml snapshot with the `locale` key deleted from `[defaults]`) |
| File verdict | REAL (test fixture doing its job) |
| Reachable from shipped entry point? | N/A — test-only; consumed by crates/product-config/tests/fixtures_matrix.rs:97 (`missing_defaults_key_is_reported`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff vs sibling fixture; python3 schema `required` extraction; sed quote verification; test execution UNVERIFIED (disk full; baseline covers package) |

## 1. Purpose / consumer / cross-check
Purpose: rule §3.3 required fields — a missing key inside an existing table. The fixture deletes `locale = "en-US"` from `[defaults]` while keeping its explanatory comment, so the mutation is a deletion, not a value change. Fixture L104-108 (verbatim):
```
theme = "dark"
# Default locale: must match locales/<tag>.toml. en-US is the reference
# catalog (§6.3) and therefore always available.
# Default playback profile id: must match profiles/<id>.toml (§7.4).
profile = "default"
```
(NB: the comment above the deleted key remains — a deliberate trap shape: comment present, key absent.)
Consumer (fixtures_matrix.rs:97-102) asserts exactly one diagnostic on `defaults.locale` with "required key is missing". Cross-check vs product.schema.json (python3): `defaults.required = ['theme', 'locale', 'profile']` — the fixture removes one of exactly the three required keys. Consistent.

## 2. What actually works (verified)
— Deletion present and isolated (diff vs sibling: only the `locale` line and the sibling's own limits mutations).
— The schema's required list contains the removed key.

## 3. Findings
F-001 [S4]
Where: whole file (stale snapshot debt, same as siblings)
What is wrong: stale product.toml base (display_version "0.1.0-dev", no `[web]`); no test impact. Confidence: HIGH — Verification: diff.

## 4. Fixture verdict
REAL: single-key deletion, schema-consistent, exact-key assertion.

## 5. Open questions
- None.
