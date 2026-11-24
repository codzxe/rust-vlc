# AUDIT: crates/localization/tests/fixtures/locales-plural-ru/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines | 7 (6 keys, doc 07 §6.2 example shapes) |
| File verdict | REAL |
| Reachable | Test fixture — the reference half of the `locales-plural-ru` pair; loaded by `load_pair` in audit_proof_matrix.rs:40-62 for `rule5_missing_plural_categories_are_flagged`; byte-equality with the canonical reference enforced by `fixture_reference_catalogs_are_identical_across_directories` (350-368); passes in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The shared canonical reference catalog copied into every `locales-*` fixture directory (cmp-verified byte-identical to `locales-good/en-US.toml` and to `locales-duplicate/en-US.toml`):
```
"fixture.simple"   = "Plain text"
"fixture.greeting" = "Hello, {name}!"
"fixture.progress" = "Buffering… {percent, number, percent}"
"fixture.source"   = "Opening {source}"
"fixture.items"    = "{count, plural, one {# item} other {# items}}"
"fixture.two"      = "Second key"
```
It is deliberately CONFORMANT (en-US plural set one/other; placeholder shapes are the doc 07 §6.2 examples) so that the ru.toml sibling is the sole source of issues — the rule-5 test asserts exactly one PluralNonConformance, so any defect introduced into this reference would break that count. The cross-directory identity test (350-368) re-reads this file and asserts byte-equality with the canonical copy plus a clean parse — a drift tripwire for all seven fixture directories at once. Zero findings — checked: key grammar (2-segment dotted kebab), placeholder-name legality, plural conformance for en (one/other present), no duplicate keys, byte-identity proven by cmp (exit 0).

## Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixture_reference_catalogs_are_identical_across_directories | YES | byte-compare of this file against the canonical reference across all 7 dirs + clean re-parse |
| rule5_missing_plural_categories_are_flagged (via load_pair) | YES | this file must load as the en-US half; its conformance is what isolates ru.toml as the sole violator |
