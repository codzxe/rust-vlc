# AUDIT: crates/localization/tests/fixtures/locales-missing/de.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 5 / n/a (test fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test input; consumed by tests/audit_proof_matrix.rs `rule1_missing_keys_are_flagged_exactly` via `load_pair("locales-missing")` + `audit_missing_keys` + `catalog_coverage`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | byte read (python3, 255 bytes, 4 keys); diff vs locales-good/de.toml (exactly fixture.greeting + fixture.items removed); baseline cargo-test-workspace.txt:2879-2976 (localization exit 0; rule1_missing_keys_are_flagged_exactly passes). |

## 1. One-paragraph verdict
Precise RULE 1 fixture: the complete de.toml minus exactly the two documented msgids (`fixture.greeting`, `fixture.items`). The consuming test asserts the missing-key list is exactly those two (order-sensitive Vec equality), coverage 4/6, and the combined entry point returning exactly 2 MissingKey issues. Nothing sloppy: the two removed keys include the placeholder message and the plural message, so the coverage metric is exercised on non-trivial shapes.

## 2. What actually works (verified)
— verified by diff against locales-good/de.toml: the ONLY difference is the removal of the fixture.greeting and fixture.items lines (4 remaining keys are byte-identical to the good fixture).
— verified by baseline: `rule1_missing_keys_are_flagged_exactly` passes — asserts keys == [fixture.greeting, fixture.items], coverage.covered == 4, reference_total == 6, percent ≈ 400/6, combined == 2 MissingKey issues.

## 3. Findings
None. Checked: exactly 2 keys missing (matches header comment); remaining 4 keys byte-identical to the control fixture; en-US.toml reference in the same dir has all 6 keys; no extra/typo'd keys that would produce unexpected diagnostics.

## 4. Per-function table
N/A (fixture).
## 5. Test honesty
`rule1_missing_keys_are_flagged_exactly`: YES — order-sensitive exact Vec assertion on the two missing keys plus exact coverage numbers; any extraction or diffing bug (duplicates, order, count) fails it.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "RULE 1 violation — missing fixture.greeting and fixture.items" | fixture line 1 | REAL | diff vs control fixture; baseline rule1 test asserts exactly these two |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
None (TOML test data).
## 9. Open questions for the next agent
None.
