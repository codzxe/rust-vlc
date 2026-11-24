# AUDIT: crates/localization/tests/fixtures/locales-placeholder/de.toml
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines | 8 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/audit_proof_matrix.rs:174-200 (`rule3_placeholder_mismatch_is_flagged_exactly`), passed in baseline (14/14 audit_proof_matrix) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | cat fixture + companion en-US.toml; sed test body; cross-check of every key's placeholder set against the reference |

## 1. Verdict
Exact implementation of doc 07 §6.4's own Rule-3 example: the reference
(locales-placeholder/en-US.toml:4) carries
`"Buffering… {percent, number, percent}"` and this fixture's
`"fixture.progress" = "Puffer… {percent}"` (:5) drops the number/percent
type — a placeholder-set mismatch and nothing else. Verified the other five
keys' placeholder sets match the reference exactly ({name}, {source}, the
plural {count} with German's one/other categories — no accidental Rule-5 or
extra Rule-3 issues), which is what the test's strict
`combined.len() == 1` assertion requires. The consuming test asserts the
single issue's file, key, expected ("number,percent"), actual ("simple"),
and the rendered diagnostic text — exact-count assertions throughout, the
strongest fixture form.

## 2. Findings
None. Zero findings — checked: all 6 keys' placeholder sets compared
key-by-key against the reference; exactly one intentional mismatch
(fixture.progress), matching the test's exact-count and detail asserts.
