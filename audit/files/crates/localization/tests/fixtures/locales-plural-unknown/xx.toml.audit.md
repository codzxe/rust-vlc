# AUDIT: crates/localization/tests/fixtures/locales-plural-unknown/xx.toml
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 10 / 6 message keys |
| File verdict | REAL (negative-control fixture proving the DOCUMENTED rule-5 leniency) |
| Reachable from shipped entry point? | NO — test fixture; consumed only by crates/localization/tests/audit_proof_matrix.rs:298-308. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read tool (byte-safe); consumer test read; baseline: audit_proof_matrix 14/14 pass. |

## 1. One-paragraph verdict
Exactly what its header says: the rule-5 negative control for a language outside the Phase-1 plural table. "xx" is unknown, so `audit_plural_conformance` enforces only the mandatory `other` arm and tolerates the extra `one` arm — the fixture carries `one`/`other` arms and must produce ZERO issues, proving both the leniency and that the lenient path does not accidentally go strict. The leniency itself is the S4 recorded in my audit.rs.audit.md (F-003); this fixture honestly pins the current behavior.

## 2. What actually works (verified)
- Fixture content (Read tool, byte-safe, lines 8):
```
"fixture.items"    = "{count, plural, one {# xx} other {# xx}}"
```
- Consumer: audit_proof_matrix.rs:298-308 `rule5_unknown_language_enforces_only_the_parse_level_other` — asserts `xx.locale() == "xx"`, `audit_plural_conformance(&xx).is_empty()`, and the combined `audit_catalogs` over the fixture dir is empty. Baseline pass (14/14).
- The five other keys mirror the locales-good canonical set (simple/greeting/progress/source/two) so rule 1/2/3 pass too — the combined-empty assertion covers all rules.

## 3. Findings
None. Zero-finding note (spec §12): checked grammar (all keys 2-segment dotted lowercase), placeholder parity with the en-US-shaped reference (fixture.greeting {name}, fixture.progress {percent, number, percent}, fixture.source {source} — matching locales-good/en-US.toml), plural arm shape, and consumer coverage.

## 4-9. Per-function table / tests / docs / dead code / deps / questions
n.a. (data fixture). Test honesty: the consumer test WOULD FAIL if the unknown-language path went strict (extra `one` arm flagged) or if rule 1-3 regressed (combined non-empty). Related doc claim: audit.rs:222-225 documents this exact leniency — consistent.
