# AUDIT: crates/product-config/tests/fixtures/migrations-inverted.toml
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 227 / n.a. (TOML fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; loaded only by crates/product-config/tests/semantic.rs:105-117. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff vs product.toml (exit 1); Read tool bytes (display-safe); baseline: semantic 17/17 pass. |

## 1. One-paragraph verdict
Correct dual-purpose fixture: one inverted migration entry (`from = 3, to = 2`) that must trip BOTH the `from < to` rule and the schema_version↔max-target desync (max `to` = 2 vs schema_version = 1). The consumer test asserts both diagnostics, so the fixture exercises rule interaction, not just one branch.

## 2. What actually works (verified)
- Mutation verified (Read tool, byte-safe; fixture lines 224-226):
```
[migrations]
# entries = []
entries = [{ from = 3, to = 2, summary = "inverted", migrator = "m" }]
```
- Consumer: semantic.rs:105-117 `migrations_inverted_entry_is_rejected` — one `migrations.entries.0.from` diagnostic (rule "from must be < to") + one `schema_version` diagnostic (rule "highest migration target"). Baseline pass.

## 3. Findings
F-001 [S4]
Where: whole file vs tests/fixtures/README.md:3-4
What is wrong: same stale-base issue as the other 39 fixtures (pre-0.1.3 product.toml copy + mutation; README's "exactly one documented mutation" regeneration contract is false — diff shows the era gap: display_version 0.1.0-dev, min 480x320, streaming-http false, no [web.size_budgets]).
Confidence: HIGH — Verification: diff.

## 4-9. Per-function table / tests / docs / dead code / deps / questions
n.a. (data fixture). Note for the crate owner: the inline-table form `entries = [{…}]` differs from the documented future contract `[[migrations.entries]]` (fixture line 217 comment) — the validator must accept both shapes; semantic.rs only proves the inline form.
