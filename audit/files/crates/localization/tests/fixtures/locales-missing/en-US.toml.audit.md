# AUDIT: crates/localization/tests/fixtures/locales-missing/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines | 9 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; reference half of the `locales-missing` pair, consumed by tests/audit_proof_matrix.rs:70-100 (`rule1_missing_keys_are_flagged_exactly`), passed in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | cat fixture + companion de.toml; sed test body; key-set diff by eye (reference 6 keys vs catalog 4) |

## 1. Verdict
The 6-key reference half of the Rule-1 (missing-key) fixture pair. The
companion de.toml drops exactly `fixture.greeting` and `fixture.items`
(verified by comparing the two files' key sets), and the consuming test
asserts the EXACT key list (Vec equality, not .any()), the coverage metric
(4 covered / 6 total, percent within 1e-4 of 400/6), and that the combined
`audit_catalogs` entry point reports exactly 2 issues, both MissingKey.
Message shapes exercise the doc 07 §6.2 subset (plain, {name}, typed
{percent, number, percent}, plural with #). Identical in content to the
locales-placeholder reference — the pairs share one reference shape, which
is consistent, not drift (verified byte-comparable key sets).

## 2. Findings
None. Zero findings — checked: 6-key reference well-formed (all keys quoted,
all message shapes valid subset forms); companion drop set is exactly the
two asserted keys; no orphan keys in de.toml (all 4 present keys exist in
the reference, so Rule 2's orphan check stays silent — required by the
test's `combined.len() == 2`).
