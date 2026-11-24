# AUDIT: crates/localization/tests/fixtures/locales-orphan/de.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Lines / keys | 8 / 7 keys (6 reference + 1 orphan) |
| File verdict | REAL (fixture doing its one job) |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/audit_proof_matrix.rs:154-166 (rule2_orphan_keys_are_flagged_exactly) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | `cargo test -p localization --test audit_proof_matrix` -> 14/14 pass (my run, includes the orphan rule); cat/md5sum verification |

## 1. Verdict
Exactly what the fixture contract demands: the reference key set plus one extra key `"fixture.orphan" = "Verwaister Schlüssel"` — the cross-file orphan that rule 2's second half must flag. The consuming test asserts exactly one `OrphanKey { file: "de.toml", key: "fixture.orphan" }` both via the individual function and the combined audit_catalogs entry point; verified passing in my run (14/14). The other 6 keys match the reference's placeholders/plural arms exactly, so no other rule fires — the mutation is properly isolated.

## 2. What actually works (verified)
— `cargo test -p localization --test audit_proof_matrix` -> 14 passed (includes `rule2_orphan_keys_are_flagged_exactly ... ok`); md5sum: all 8 fixture dirs share a byte-identical en-US.toml (49a14b21…), so the orphan is the only delta in this pair.

## 3. Findings
None. Checked: key set = reference + exactly one orphan (verified against locales-good/en-US.toml); placeholders match the reference for all shared keys ({name}, {percent, number, percent}, {source}, plural arms one/other); no duplicate keys; valid TOML with quoted keys; the de locale matches the file name.

## 9. Open questions: none.
