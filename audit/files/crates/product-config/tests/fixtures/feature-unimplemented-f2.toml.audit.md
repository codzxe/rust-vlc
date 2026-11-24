# AUDIT: crates/product-config/tests/fixtures/feature-unimplemented-f2.toml (fixture)
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines | 240 (product.toml snapshot; single functional mutation at L153) |
| File verdict | REAL (test fixture doing its job) |
| Reachable from shipped entry point? | N/A — test-only; consumed by crates/product-config/tests/semantic.rs:216 (`f2_exposed_but_unimplemented_feature_is_rejected`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep/sed line verification; diff vs sibling fixture range-limits-multi.toml; test execution UNVERIFIED (disk full — see log; baseline covers the package) |

## 1. Purpose / consumer / cross-check
Purpose: prove rule F2 — an exposed (`= true`) feature whose engine registry entry says unimplemented is a validation error. The fixture is a product.toml snapshot whose only functional mutation is `streaming-http = true`. Consumer (semantic.rs:216-230) injects a synthetic `FeatureRegistry::from_status([("core-playback", true), ("streaming-http", false)])` and asserts exactly one diagnostic on `features.streaming-http` carrying "rule F2" and expected "= false".
Fixture L152-154 (verbatim):
```
library = false                 # Phase 4
streaming-http = true

```
Consumer (semantic.rs:218-222, verbatim):
```
    let inputs = SemanticInputs {
        feature_registry: Some(FeatureRegistry::from_status([
            ("core-playback", true),
            ("streaming-http", false),
        ])),
```
Cross-check vs config-validator semantics: product.toml §2.7 states "Rule F2: enabling a feature requires its implementation to exist AND pass its gate … config alone enables nothing" — the fixture+registry pair is exactly that rule's negative case. The test is honest: the violation lives in the (file, registry) pairing, and a sibling test (`f1_f2_are_deferred_when_no_registry_is_supplied`) pins the deferred behavior when no registry is supplied, so the seam's limits are also documented.

## 2. What actually works (verified)
— The mutation is present and isolated: diff vs range-limits-multi.toml shows only `streaming-http = true` (+ blank line) and the limits values (which belong to the sibling fixture's own mutation). tomllib parses the file (same structure as the sibling, verified there).
— The asserted key (`features.streaming-http`) exists in the fixture at L153 with value `true`, matching `d.found`-style expectations.

## 3. Findings
F-001 [S4]
Where: whole file (stale snapshot)
What is wrong: same stale-snapshot debt as the sibling fixture (display_version "0.1.0-dev", min_width 480/320, no `[web]` section). Note the irony: current product.toml legitimately ships `streaming-http = true` (w31.3), so this fixture's mutated value now equals the real file — the test only stays meaningful because the registry is injected as unimplemented. If someone "fixes" the fixture to `false` the test breaks; a README note in tests/fixtures/README.md (the documented fixture ↔ rule map) should record that the F2 violation is registry-relative. Confidence: HIGH — Verification: diff + read.

## 4. Fixture verdict
REAL: single-purpose mutation, honest consumer asserting rule id + expected value + exact key; the deferred-seam behavior is separately pinned.

## 5. Open questions
- None.
