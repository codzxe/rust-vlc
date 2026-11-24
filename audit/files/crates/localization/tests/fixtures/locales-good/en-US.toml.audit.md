# AUDIT: crates/localization/tests/fixtures/locales-good/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Lines / keys | 7 / 6 keys |
| File verdict | REAL (the negative-control reference fixture) |
| Reachable from shipped entry point? | NO — test fixture; canonical reference for all 8 fixture dirs; consumed by audit_proof_matrix.rs (rule1 coverage 4/6 math, rule4 negative control, a_conformant_tree_produces_zero_issues) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | `cargo test -p localization --test audit_proof_matrix` -> 14/14 pass (my run); md5sum across all 8 fixture dirs; cat verification |

## 1. Verdict
The canonical 6-key reference catalog: plain text, {name} placeholder, the doc-07 §6.4 `{percent, number, percent}` example, {source}, a one/other plural, and a second plain key. It is the negative control that `a_conformant_tree_produces_zero_issues` proves clean, the base for rule 1's coverage arithmetic (locales-missing covers 4/6), and the reference used by rule 4's resolved-refs control. Byte-identical copies (md5 49a14b21…) live in all 8 fixture directories, enforced by `fixture_reference_catalogs_are_identical_across_directories` — verified passing in my run (14/14).

## 2. What actually works (verified)
— `cargo test -p localization --test audit_proof_matrix` -> 14 passed; md5sum of 8 en-US.toml copies identical.

## 3. Findings
None. Checked: valid TOML; placeholder sets consistent (the de translations in the sibling fixtures match these exactly); plural arms = one+other (the en CLDR set, so rule 5 passes); key names parse as message keys; no duplicates; en-US is the REFERENCE_LOCALE the loaders require.

## 9. Open questions: none.
