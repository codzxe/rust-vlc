# AUDIT: crates/localization/tests/fixtures/locales-duplicate/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 7 (reference catalog fixture: 6 keys) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only asset; consumed by tests/audit_proof_matrix.rs `rule2_duplicate_keys_fail_at_parse_level` (L106) and `rule2_duplicate_keys_surface_through_the_combined_audit`, plus the `fixture_reference_catalogs_are_identical_across_directories` guard (L350). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/audit_proof_matrix-413ebc99acdad849 rule2_duplicate` → 2 passed; `... fixture_reference_catalogs` → 1 passed; `diff` vs locales-plural-unknown/en-US.toml → identical; rg doc-07 example cross-ref |

## 1. One-paragraph verdict
This is the well-formed reference side of the duplicate-key fixture pair: the actual rule-2 violation lives in the sibling de.toml (which declares `"fixture.two"` twice), while this en-US.toml must load cleanly so the test can assert that exactly ONE catalog (de.toml) fails with a "duplicate key" parse error. I executed both consuming tests from the prebuilt binary — `rule2_duplicate` (2 passed: parse-level failure asserted exactly-once with the full catalog list rendered on mismatch, error text contains "duplicate key") and the cross-directory identity guard (1 passed: this file must be byte-identical to locales-good/en-US.toml — verified identical by my own diff too). Keys conform to the MessageKey grammar (2 segments, lowercase); the `{percent, number, percent}` value matches doc 07 §6.2's example verbatim (docs/07-config-driven-system.md:653). Zero findings.

## 2. What actually works (verified)
- Executed: `rule2_duplicate_keys_fail_at_parse_level ... ok`, `rule2_duplicate_keys_surface_through_the_combined_audit ... ok` (audit_proof_matrix binary, 2 passed / exit 0).
- Executed: `fixture_reference_catalogs_are_identical_across_directories ... ok` — the guard that keeps this file in lockstep with 7 sibling directories.
- `diff locales-duplicate/en-US.toml locales-plural-unknown/en-US.toml` → identical (both are the canonical 6-key reference shape).
- Grammar: all 6 keys (`fixture.simple`, `fixture.greeting`, `fixture.progress`, `fixture.source`, `fixture.items`, `fixture.two`) pass the 2-segment lowercase rule — consistent with my 17-catalog scan in the key.rs audit.
- Doc example provenance: "Buffering… {percent, number, percent}" = docs/07-config-driven-system.md:653 verbatim.

## 3. Findings
None. Checked: (a) file loads as the reference in executed tests; (b) byte-identity guard executed; (c) key grammar conformant; (d) plural arms (one/other) match English CLDR categories (the en-US reference must not itself trip rule 5 — the combined-audit test passing proves it); (e) duplicate mutation correctly isolated in the sibling de.toml, not here.

## 4-9. (short audit)
Per-function table: N/A (data). Test honesty: high — the tests assert exact failure counts, error substrings, and render the whole loaded-catalog list on failure. Doc claims: header comment "the reference catalog shape (doc 07 §6.2 examples)" verified against doc 07:653. Dead code: none. Dependencies: none. Open questions: none.
