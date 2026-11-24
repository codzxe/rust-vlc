# AUDIT: crates/product-config/tests/fixtures/cross-field-schema-version.toml
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 227 / n.a. (TOML fixture) |
| File verdict | REAL (fixture does its job; consumed by a passing strict test) |
| Reachable from shipped entry point? | NO — test fixture; loaded only by crates/product-config/tests/semantic.rs:58-65 via common::load_fixture. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff vs product.toml (exit 1); Read tool bytes (display-safe); baseline: semantic 17/17 pass. |

## 1. One-paragraph verdict
A valid structural, invalid semantic fixture: the doc-07 §3.3 cross-field rule "schema_version == max(migrations.entries.to) (or 1 when empty)". Mutation present and verified. The base copy is stale (see finding) but that does not affect the rule under test.

## 2. What actually works (verified)
- Mutation verified (Read tool, byte-safe):
```
schema_version = 2
```
(fixture line 10; root product.toml:10 has `schema_version = 1`; the fixture's `[migrations]` table has no entries, so expected max target = 1).
- Consumer: semantic.rs:58-65 `cross_field_schema_version_must_match_migrations` asserts exactly one `schema_version` diagnostic, rule contains "highest migration target", found "2", expected "1". Baseline pass (17/17).

## 3. Findings
F-001 [S4]
Where: whole file vs tests/fixtures/README.md:3-4
What is wrong: the README says each fixture is the repo-root product.toml with exactly one documented mutation; this fixture is a pre-0.1.3 copy (display_version "0.1.0-dev" vs product.toml "0.1.3"; min_width 480 vs 940; streaming-http false vs true; no [web.size_budgets]; pre-ADR-001-acceptance comments) plus the mutation. 39/40 fixtures share the same stale base (rg count). Test remains valid; the regeneration contract is false.
Confidence: HIGH — Verification: diff + rg.

## 4-9. Per-function table / tests / docs / dead code / deps / questions
n.a. (data fixture). Test honesty covered in semantic.rs.audit.md. No doc claims beyond the README map (verdict FALSE, above). No dead code. No deps.
