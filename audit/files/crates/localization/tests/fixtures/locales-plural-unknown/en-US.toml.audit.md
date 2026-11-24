# AUDIT: crates/localization/tests/fixtures/locales-plural-unknown/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 7 (reference catalog fixture: 6 keys — byte-identical to locales-duplicate/en-US.toml) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only asset; consumed by tests/audit_proof_matrix.rs `rule5_unknown_language_enforces_only_the_parse_level_other` (L298-308) and the `fixture_reference_catalogs_are_identical_across_directories` guard (L350, lists "locales-plural-unknown"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/audit_proof_matrix-413ebc99acdad849 rule5_unknown` → 1 passed; `... fixture_reference_catalogs` → 1 passed; `diff` vs locales-duplicate/en-US.toml → identical |

## 1. One-paragraph verdict
The reference side of the rule-5 negative-control pair: the sibling xx.toml is an UNKNOWN locale ("xx" absent from the Phase-1 plural table), and the executed test proves the audit degrades to parse-level-only enforcement for it (`audit_plural_conformance(&xx).is_empty()` and the COMBINED `audit_catalogs` over this directory returns zero issues — meaning this en-US.toml itself contributes no violations: its `one`/`other` plural arms conform for English). Both tests executed from the prebuilt binary, both pass. The file is byte-identical to the canonical locales-good reference (verified by diff AND by the executed identity guard). Zero findings.

## 2. What actually works (verified)
- Executed: `rule5_unknown_language_enforces_only_the_parse_level_other ... ok` — asserts `xx.locale().as_str() == "xx"`, empty per-catalog issues, and empty combined audit for the whole directory (this file included).
- Executed: `fixture_reference_catalogs_are_identical_across_directories ... ok` (this directory is in the guard's list, L360).
- `diff` vs locales-duplicate/en-US.toml → identical.
- Key grammar conformant (6 keys, 2 segments each — see the 17-catalog scan in the key.rs audit).

## 3. Findings
None. Checked: (a) both consuming tests executed green; (b) byte-identity with the canonical reference enforced by a test, not convention; (c) grammar conformant; (d) the file's own plural arms don't contaminate the negative control (combined-audit-empty assertion covers it).

## 4-9. (short audit)
Per-function table: N/A (data). Test honesty: the test pins the exact degradation semantics for unknown locales (only the parse-level `other` arm enforced) — a real behavioral contract, not a smoke check. Doc claims: header "reference catalog shape (doc 07 §6.2 examples)" — same verification as the duplicate fixture (docs/07-config-driven-system.md:653). Dead code: none. Dependencies: none. Open questions: none.
