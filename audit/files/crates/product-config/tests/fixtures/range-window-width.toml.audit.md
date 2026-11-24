# AUDIT: crates/product-config/tests/fixtures/range-window-width.toml (fixture)
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines | 240 (product.toml snapshot; single functional mutation at L64) |
| File verdict | REAL (test fixture doing its job) |
| Reachable from shipped entry point? | N/A — test-only; consumed by crates/product-config/tests/fixtures_matrix.rs:174 (`range_window_width_is_rejected_with_line`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff vs sibling fixture; python3 schema-range extraction; sed quote verification; test execution UNVERIFIED (disk full; baseline covers package) |

## 1. Purpose / consumer / cross-check
Purpose: rule §3.3 ranges with a LINE-PINNED diagnostic. Mutation (L63-64, verbatim):
```
[window]
width = 100
```
Consumer (fixtures_matrix.rs:174-182) asserts one diagnostic on `window.width` with "must be >= 480", "<= 3840", found "100", expected "480..=3840", and `d.line == marker_line(&src, "width = 100")` — the diagnostic must point at the exact source line, the strongest form of the contract. Cross-check vs product.schema.json (python3 extraction): `width` minimum 480, maximum 3840 — the asserted bounds are the schema's bounds, not independent constants. `height = 720`, `min_width = 480` stay in-range so the fixture isolates one violation (plus whatever cross-field rule fires on min_width, which the test does not assert — `one()` only pins window.width).

## 2. What actually works (verified)
— Mutation present and isolated (diff vs sibling: L64 + the sibling's own limits block).
— Schema bounds match the test's asserted strings exactly.

## 3. Findings
F-001 [S4]
Where: whole file (stale snapshot — display_version "0.1.0-dev", min_width 480/320 base, no `[web]`)
What is wrong: same stale-copy debt as siblings; no test impact. Note the schema's `min_width` minimum is 320 (python3 extraction), while product.toml's own comment says "schema: min_width < width" — the cross-field rule, owned by the semantic layer, is what actually enforces the floor relationship; the fixture does not exercise it. Confidence: HIGH — Verification: diff + schema read.

## 4. Fixture verdict
REAL: single-purpose mutation, line-pinned assertion (the honest kind — it fails if the diagnostic loses its line number).

## 5. Open questions
- None.
