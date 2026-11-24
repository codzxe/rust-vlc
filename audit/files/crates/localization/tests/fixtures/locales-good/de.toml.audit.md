# AUDIT: crates/localization/tests/fixtures/locales-good/de.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 7 / n/a (test fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test input; consumed by tests/audit_proof_matrix.rs (load_pair("locales-good") — the reference pair for rule1/rule4 and the negative control) and by refs tests as the reference catalog. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | byte read (python3, 358 bytes, 6 keys); cross-read of sibling en-US.toml (6 keys, same msgids); baseline cargo-test-workspace.txt:2879-2976 (localization exit 0: 45 unit + audit_proof_matrix rule1/rule4 pass with this fixture as reference). |

## 1. One-paragraph verdict
A complete German translation over the same 6 msgids as its en-US.toml reference — the negative control for RULE 1 (no missing keys) and the reference catalog for RULE 4 (all refs resolve). Includes real ICU message syntax: `{name}` placeholder, `{percent, number, percent}` number format, and a `{count, plural, one {# Element} other {# Elemente}}` plural — so the fixture also exercises non-trivial message shapes, not just flat strings.

## 2. What actually works (verified)
— verified against en-US.toml: identical msgid set {fixture.simple, fixture.greeting, fixture.progress, fixture.source, fixture.items, fixture.two} — completeness claim holds.
— verified by baseline: rule1's negative path and rule4's `audit_references(&reference, &refs)` clean pass both use this pair; the rule4 test asserts `refs.len()==2` extracts from unresolved.toml against THIS catalog, and the resolved.toml control yields 0 issues.

## 3. Findings
None. Checked: all 6 reference msgids present (no missing keys); values non-empty; German content is genuine (Einfacher Text / Hallo, {name}! / Öffne {source} / plural Element/Elemente); plural syntax well-formed; header comment ("complete, conformant translation (negative control)") matches.

## 4. Per-function table
N/A (fixture).
## 5. Test honesty
Serving as reference + negative control, this fixture would surface breakage in both directions: a missing key here flips rule1 counts (test asserts exact 2-key Vec for the locales-missing sibling and coverage 4/6), and a broken reference shape breaks rule4's exact-match assertions.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a complete, conformant translation (negative control)" | fixture line 1 | REAL | 6/6 keys vs en-US.toml; baseline rule1/rule4 pass |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
None (TOML test data).
## 9. Open questions for the next agent
Do the ICU-message shapes ({percent, number, percent}, plural) get FORMAT-validated anywhere (rust-i18n/fluent-style parsing), or only key-completeness-checked? Key-level audit only is observed here — the formatter consumer is out of my file scope.
