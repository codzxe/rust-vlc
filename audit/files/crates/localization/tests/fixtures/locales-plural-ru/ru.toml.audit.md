# AUDIT: crates/localization/tests/fixtures/locales-plural-ru/ru.toml
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines | 7 (full 6-key reference shape, one plural defect) |
| File verdict | REAL |
| Reachable | Test fixture — consumed by audit_proof_matrix.rs `rule5_missing_plural_categories_are_flagged` (254-278); passes in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
Rule-5 (plural conformance) violation fixture for Russian, and it is a REALISTIC one — not a synthetic arm-stripping: Russian CLDR cardinal categories are one/few/many/other (plural.rs:80 `"ru" | "pl" => ...`), and the fixture carries only one/other:
```
# Fixture: RULE 5 violation — ru requires one/few/many/other;
# this translation only carries one/other.
...
"fixture.items"    = "{count, plural, one {# элемент} other {# элемента}}"
```
The defect mirrors what a real translator submits: `элемента` is the correct FEW form (2-4 элемента), here misplaced into `other`, and the MANY form (5+ элементов) is missing entirely — so the file would render wrong numbers even where it has coverage. The consuming test asserts locale "ru", exactly one issue, file ru.toml, key fixture.items, and that the detail names BOTH missing categories ("few" and "many"), plus the combined `audit_catalogs` entry point surfacing `PluralNonConformance`. Passes in baseline. Mutation reasoning: delete the rule-5 check → test fails; flag only one missing category → the two-category detail assert fails. Cross-directory en-US identity pinned by `fixture_reference_catalogs_are_identical_across_directories` (cmp verified against locales-good: IDENTICAL). Zero findings — checked: all 6 reference keys present (no accidental rule-1/rule-2/rule-3 noise — the test asserts exactly ONE issue, so any extra defect would fail it), placeholder sets match the reference ({name}, {source}, {percent, number, percent}, {count, plural, …}), header comment accurate.

## Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rule5_missing_plural_categories_are_flagged | YES | pins issue count, file, key, and both missing category names in the detail |
