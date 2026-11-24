# AUDIT: crates/localization/tests/fixtures/locales-duplicate/de.toml
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines | 8 (6 unique keys + 1 duplicated) |
| File verdict | REAL |
| Reachable | Test fixture — consumed by audit_proof_matrix.rs `rule2_duplicate_keys_fail_at_parse_level` (106-140) and `rule2_duplicate_keys_surface_through_the_combined_audit` (143-151); both pass in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
Rule-2 (in-file duplicate) violation fixture, minimal and precise: the SAME key `"fixture.two"` is defined twice with different values, which must fail at TOML-parse level ("duplicate detection is parse-level for free", doc 07 §6.2):
```
"fixture.two"      = "Zweiter Schlüssel"
"fixture.two"      = "Doppelter Schlüssel"
```
(byte-verified lines 7-8). The rest of the file carries the full 6-key reference shape so the ONLY defect is the duplicate. The consuming tests assert: exactly one load failure naming de.toml with "duplicate key" in the error, the sibling en-US.toml still loads (partial-failure isolation), and the combined `audit_catalogs` surfaces it as `CatalogUnreadable` — the exact entry point CI would call. Both tests pass (baseline). Mutation reasoning: if load_locales_dir silently kept the last duplicate, both tests fail; if it aborted the whole directory load, the en-US-still-loads assert fails. The `fixture_reference_catalogs_are_identical_across_directories` test (350-368) pins this directory's en-US.toml byte-equal to the canonical one (cmp verified: IDENTICAL). Zero findings — checked: TOML validity (invalid by intent, proven by the passing test), key grammar, reference-shape completeness, sibling isolation, cross-directory reference identity.

## Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rule2_duplicate_keys_fail_at_parse_level | YES | pins failure count, file name, error text, and en-US survival |
| rule2_duplicate_keys_surface_through_the_combined_audit | YES | pins the combined-audit issue type + rendered message |
