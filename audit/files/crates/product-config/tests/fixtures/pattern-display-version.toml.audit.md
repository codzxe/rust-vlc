# AUDIT: crates/product-config/tests/fixtures/pattern-display-version.toml (fixture)
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines | 240 (product.toml snapshot; single functional mutation at L27) |
| File verdict | REAL (test fixture doing its job) |
| Reachable from shipped entry point? | N/A — test-only; consumed by crates/product-config/tests/fixtures_matrix.rs:246 (`pattern_display_version_is_rejected`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep/sed verification; diff vs sibling fixture; schema pattern read from product.schema.json:32-35; test execution UNVERIFIED (disk full; baseline covers package) |

## 1. Purpose / consumer / cross-check
Purpose: rule §3.3 patterns — `identity.display_version` must be semver-shaped. Fixture mutation (L27, verbatim):
```
display_version = "version one"
```
Consumer (fixtures_matrix.rs:246-252) asserts exactly one diagnostic on `identity.display_version` containing "must match the pattern" with found `"version one"`. Cross-check vs validator semantics (product.schema.json:32-35, verbatim):
```
        "display_version": {
          "type": "string",
          "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+(-[A-Za-z0-9.]+)?$",
```
`"version one"` violates that regex on its face (no digit-dot shape), so fixture, test, and schema agree. The current product.toml value "0.1.3" satisfies the same pattern, confirming the rule is not over-strict for the real file.

## 2. What actually works (verified)
— Mutation present and isolated (diff vs sibling: only L27 + the limits block that belongs to the sibling's own mutation).
— The asserted key/value pair exists exactly once in the fixture.

## 3. Findings
F-001 [S4]
Where: whole file (stale snapshot)
What is wrong: same stale product.toml-copy debt as the sibling fixtures (display_version base "0.1.0-dev", min_width 480/320, no `[web]`). Test impact none (asserts only the one key). Confidence: HIGH — Verification: diff.

## 4. Fixture verdict
REAL: single-purpose mutation, schema-consistent, honest exact-key assertion.

## 5. Open questions
- None.
