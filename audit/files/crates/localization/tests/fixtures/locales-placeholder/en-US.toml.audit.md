# AUDIT: crates/localization/tests/fixtures/locales-placeholder/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines / keys | 7 / 6 message keys |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/audit_proof_matrix.rs:173-200 (`rule3_placeholder_mismatch_is_flagged_exactly`) and :357 (`fixture_reference_catalogs_are_identical_across_directories`); executed green this session |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | sha256sum x8 en-US.toml fixtures (all identical a263d97a…); `cargo test -p localization --test audit_proof_matrix --offline rule3` -> 1 passed, exit 0; full suite -> 14 passed, exit 0; `rg 'select,' tests/fixtures` -> 0 hits; `rg PlaceholderMismatch` -> 2 files; cat -A byte check; sed verification of every quote |

## 1. One-paragraph verdict
This is the REFERENCE half of the Rule-3 (placeholder validation) violation
pair: it defines the placeholder contract (name -> type tag) that the
companion de.toml violates on exactly one key. Every placeholder it declares
is grammatical under message.rs's v1-subset parser, and its
`fixture.progress`/`fixture.source` values are verbatim copies of doc 07
§6.2's example messages. The consuming test executes green and pins the
exact issue (file, key, expected tag, actual tag, count) — an honest,
strong fixture. Two coverage debts sit around it, not in it: rule 3's
"names" half is never mutation-pinned anywhere, and the `{name, select, …}`
form of the grammar has no fixture at all.

## 2. What actually works (verified)
— `cargo test -p localization --test audit_proof_matrix --offline rule3`
-> `test rule3_placeholder_mismatch_is_flagged_exactly ... ok` (1 passed,
exit 0); full `--test audit_proof_matrix` -> 14 passed, 0 failed, exit 0.
— sha256 of all 8 fixture dirs' en-US.toml -> byte-identical
(a263d97a383e32172c257390473365af74d4cc8f130e04fb2d96444bd1ef7b88),
independently corroborating the in-tree identity test.

## 3. Findings
F-001 [S3] Rule 3's contract "names + types must equal" is only type-pinned;
the names half is mutation-dead.
Where: crates/localization/src/audit.rs:168-171 (contract), tests/audit_proof_matrix.rs:173-200, src/audit.rs:447-474 (the only two PlaceholderMismatch tests)
Code (audit.rs:168-171, verbatim):
```
/// Rule 3 — Placeholder validation: for each shared key, the placeholder
/// set (names + types) must equal the reference's exactly (doc 07 §6.4:
/// `{percent, number, percent}` in en-US and `{percent}` in `de` is a CI
/// failure).
```
What is wrong: every PlaceholderMismatch test case in the repo (rg: only
audit_proof_matrix.rs and audit.rs) varies the TYPE only — reference
`{percent, number, percent}` vs catalog `{percent}`, same placeholder name
"percent". No test has a name-only mismatch (e.g. `{prozent}` vs
`{percent}`). A mutant `placeholder_set()` that erases names and keeps type
tags produces identical results on both catalogs' test data and passes all
rule-3 tests; the doc-promised name comparison is unverified behavior.
Why it matters: a translator renaming a placeholder (the most common real
rule-3 violation) would not be caught by any test if the name comparison
regressed. What correct looks like: add a fixture pair differing only in
placeholder name and assert the mismatch. Confidence: HIGH — Verification:
read + rg (mutation reasoning; no code edited).

F-002 [S4] The `{name, select, …}` form of the §6.2 grammar has zero
fixture coverage; this fixture models 3 of the 4 documented forms.
Where: this file:2-7; docs/07-config-driven-system.md:667; message.rs:7-12
Code (this file:2-7, verbatim):
```
"fixture.simple"   = "Plain text"
"fixture.greeting" = "Hello, {name}!"
"fixture.progress" = "Buffering… {percent, number, percent}"
"fixture.source"   = "Opening {source}"
"fixture.items"    = "{count, plural, one {# item} other {# items}}"
"fixture.two"      = "Second key"
```
What is wrong: the header claims "the reference catalog shape (doc 07 §6.2
examples)", and `fixture.progress`/`fixture.source` are indeed verbatim
doc 07:653-654 values — but the select form (`{name, select, …}`, doc
07:667, message.rs:7-12) appears in NO file under tests/fixtures/
(`rg 'select,' crates/localization/tests/fixtures/` -> 0 hits). Select is
pinned only by message.rs unit tests (message.rs:796, 974). Why it matters:
the fixture tree is the proof matrix for the CI audit rules; the select
type tag ("select", message.rs:123) never crosses a catalog boundary in any
test. What correct looks like: one fixture pair using a select placeholder
in en-US. Confidence: HIGH — Verification: ran rg.

F-003 [S4] audit.rs hardcodes a semantic duplicate of this fixture with no
mechanical sync.
Where: crates/localization/src/audit.rs:363-370
Code (verbatim):
```
    const REFERENCE: &str = "\
\"fixture.simple\" = \"Plain text\"
\"fixture.greeting\" = \"Hello, {name}!\"
\"fixture.progress\" = \"Buffering… {percent, number, percent}\"
\"fixture.source\" = \"Opening {source}\"
\"fixture.items\" = \"{count, plural, one {# item} other {# items}}\"
\"fixture.two\" = \"Second key\"
";
```
What is wrong: same 6 key/value pairs as this file (verified key-by-key;
whitespace differs), but the byte-identity test
(audit_proof_matrix.rs:350-364) only compares fixture directories against
each other — nothing ties the inline const to the fixture files. If the
fixtures evolve, the audit.rs unit tests keep passing on the stale copy.
Why it matters: silent test-data drift. What correct looks like: have the
unit tests read the fixture file, or extend the identity test to the const.
Confidence: HIGH — Verification: read + diff-by-inspection.

## 4. Per-key placeholder inventory (cross-check vs message.rs grammar)
| Key | Placeholders | message.rs kind (tag) | Grammar-conformant? |
|---|---|---|---|
| fixture.simple | none | — | YES (plain text) |
| fixture.greeting | {name} | Simple ("simple", message.rs:121) | YES; name matches parse_ident `[A-Za-z_][A-Za-z0-9_]*` (message.rs:579-595) |
| fixture.progress | {percent, number, percent} | NumberPercent ("number,percent", message.rs:122) | YES; verbatim doc 07:653 value |
| fixture.source | {source} | Simple | YES; verbatim doc 07:654 value |
| fixture.items | {count, plural, one {# item} other {# items}} | Plural ("plural", message.rs:124) | YES; `#` inside plural arms is grammar-legal (message.rs:19-20); one+other satisfies en CLDR set |
| fixture.two | none | — | YES |

All 4 placeholder names (name/percent/source/count) pass the identifier
grammar; 3 of the 4 §6.2 kinds exercised (select missing — F-002).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rule3_placeholder_mismatch_is_flagged_exactly (audit_proof_matrix.rs:172-200) | YES for the type comparison and integration | Asserts exact count (1), file, key, expected.contains("number,percent"), actual.contains("simple"), and the rendered text; a type-blind mutant yields 0 issues -> len assert fails. Executed this session. |
| same test, for the NAME comparison | NO | Name-only mutants survive (F-001): both catalogs use identical placeholder names on every key. |
| fixture_reference_catalogs_are_identical_across_directories (audit_proof_matrix.rs:349-368) | YES | Byte-equality assert across 8 dirs + re-parse of canonical; drift in any copy fails. Independently corroborated by sha256 this session. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the reference catalog shape (doc 07 §6.2 examples)" | this file:1 | REAL (3/4 forms) | fixture.progress/fixture.source are byte-for-byte doc 07:653-654 message values; select form absent (F-002) |
| "{percent, number, percent} in en-US and {percent} in de is a CI failure" (rule 3) | doc 07:682 (§6.4 rule 3), audit.rs:168-171 | REAL | The pair reproduces exactly this; test executed green, flags 1 PlaceholderMismatch |

## 7. Dead / duplicated / unreachable code
Fixture is dev-lane only (cargo test -p localization); not reachable from
lumen-cli/desktop release builds — correct for a fixture. Duplication:
audit.rs:363-370 inline const (F-003). Companion de.toml audited by
W11-A10 (REAL, zero findings) — consistent with this audit.

## 8. Dependencies & flags
None — plain TOML data file; no features, no build wiring.

## 9. Open questions for the next agent
None blocking. If a Wave adds a select-form fixture (F-002), also add the
name-only-mismatch pair (F-001) in the same pass.
