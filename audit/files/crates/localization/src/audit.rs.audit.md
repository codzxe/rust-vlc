# AUDIT: crates/localization/src/audit.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 514 / 11 (Display::fmt, Coverage::percent, audit_missing_keys, catalog_coverage, audit_orphan_keys, audit_placeholders, format_placeholder_set, audit_references, audit_plural_conformance, plural_conformance_detail, audit_catalogs, audit_catalogs_with_refs, catalog_error_issue) + 6 tests |
| File verdict | REAL (library logic correct and exercised by real fixture tests) — but UNREACHABLE: no crate in the tree depends on `localization`; only its own test suite calls it |
| Reachable from shipped entry point? | NO. rg over every Cargo.toml: zero `[dependencies]` on `localization`. Sole callers: crates/localization/tests/{audit_proof_matrix,seed_integrity}.rs via `cargo test --workspace`. The doc-07 §6.4 CI command `cargo xtask l10n-audit` does NOT exist (tools/xtask/src/cmd/mod.rs subcommand table: gate, schema-gen, tokens, codegen, check-architecture, check-panics, rebrand, fixtures, visual, feature-audit, config-validator, parity-report, lint-literals, check-identity, sbom, compat-matrix, web-budget — no l10n). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg searches (exit 0/1 as noted); python3 cross-check of schemas/settings.toml msgids vs locales/en-US.toml (121/121 unresolved); sed verifications (exit 0). Baseline: audit/runs/baseline/cargo-test-workspace.txt:2879-2977 `### PACKAGE localization exit_code=0` (45+14+3+2 tests). Own cargo run NOT attempted (theme-tokens link attempt already failed with `ld ... signal 7 Bus error` — sandbox disk). |

## 1. One-paragraph verdict
The audit tool is real and does what it says: five rules (missing keys, orphan keys + duplicate-keys-via-parse-error, placeholder-set equality incl. types, msgid reference completeness, CLDR plural-arm conformance), each a pure library function, combined in `audit_catalogs_with_refs`, with an honest proof-matrix test suite (14 tests, violation fixture per rule + negative controls) and a seed-integrity suite that audits the real `locales/` tree. The damning part is wiring and scope: nothing in the shipped product depends on this crate, the documented CI gate (`cargo xtask l10n-audit`, doc 07 §6.4/§9 step 6) does not exist, and the rule-4 scan actually performed covers only product.toml's 2 refs — the 121 msgid-shaped `description` fields in schemas/settings.toml are never checked against the reference catalog (they are ALL absent from it; the join is instead enforced UI-side in ui/src/screens/settings/generated.test.ts).

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:2879-2977: localization 45 lib tests (incl. audit.rs's 6), 14 audit_proof_matrix, 3 seed_integrity, 2 doc-tests — all pass, exit_code=0. seed_integrity `shipped_locales_tree_passes_every_catalog_audit_rule` runs `audit_catalogs(&repo("locales"))` on the REAL tree and `product_toml_msgids_resolve_in_the_reference_catalog` runs rule 4 on the real product.toml.
- My python3 run of the rule-4 semantics over schemas/settings.toml (MSGID_FIELDS walk + doc-07 §6.1 grammar `[a-z0-9-]+` 2–5 segments): 121 msgid refs, 121 missing from locales/en-US.toml — proving the scope gap quantitatively.

## 3. Findings
F-001 [S3]
Where: crates/localization/src/audit.rs:15-16 (doc claim) + crates/localization/tests/seed_integrity.rs:38-41 (actual shipped scope)
Code:
```
//! CI wiring (`cargo xtask l10n-audit`, doc 07 §6.4) is card 1.E2; until
//! then this crate's test suite runs the same audits on the shipped tree.
```
What is wrong: doc 07 §6.4 (docs/07-config-driven-system.md:678 "Run by `cargo xtask l10n-audit`" and :846 gate step `6  cargo xtask l10n-audit                  (C3)`) describes a CI gate that does not exist — xtask has no such subcommand (verified: rg 'l10n' tools/xtask/src → no matches). The substitute enforcement is the test suite, and its rule-4 half scans ONLY product.toml (seed_integrity.rs:39 `extract_msgid_refs_from_file(&repo("product.toml"))`, asserting `refs.len() == 2`), while doc 07 §6.4 rule 4 names `product.toml`, `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`, `themes/*`. schemas/settings.toml exists today with 121 msgid references; every one of them is unresolved against locales/en-US.toml (python3 cross-check). The audit rules themselves are correct; the claimed enforcement is 2 refs out of 123. Mitigation: the UI independently enforces the settings join (ui/src/screens/settings/generated.test.ts:251-266 checks all 121 SETTINGS_MSGIDS + enum options + section titles against the UI catalog) — but that is a different catalog (ui/src/i18n/messages.ts), not the reference catalog this file audits. Why it matters: the reference catalog is drifting from the real product (16 keys vs the UI's 1000+), and the documented CI net that would catch it is paper. What correct looks like: either land card 1.E2 (xtask l10n-audit scanning all doc-listed sources) or shrink doc 07 §6.4 to the enforced truth.
Confidence: HIGH — Verification: read + rg + python3 cross-check.

F-002 [S3]
Where: crates/localization/src/audit.rs:345-350
Code:
```
pub fn catalog_error_issue(file: &str, error: &CatalogError) -> CatalogIssue {
    CatalogIssue::CatalogUnreadable {
        file: file.to_string(),
        error: error.to_string(),
    }
}
```
What is wrong: zero callers in the entire tree (rg `catalog_error_issue` → only the definition). The doc comment says "exposed for callers that load catalogs themselves, e.g. the future `xtask l10n-audit`" — the future caller never landed. Dead public API whose only justification is a card (1.E2) that is still open. Combined with F-001 this is the wiring debt made concrete.
Confidence: HIGH — Verification: read + rg (exit 1 for callers, definition only).

F-003 [S4]
Where: crates/localization/src/audit.rs:222-239
Code:
```
/// For languages outside the Phase-1 category table only the universal
/// `other` category is required — extra arms are accepted, not flagged
/// (see [`crate::plural`] for the honest data-scope note).
```
What is wrong: rule 5 silently degrades to other-only for any language outside the 17-language table (plural.rs:76-101: en/de/es/fr/it/nl/pt, ru/pl, ar, ja/zh/ko/th/vi). A wrong plural arm set in, say, a Hindi or Arabic-dialect catalog passes the audit. Documented honestly, and no such catalogs ship yet (locales/ = en-US only), so blast radius today is zero.
Confidence: HIGH — Verification: read + ls locales/.

F-004 [S4]
Where: crates/localization/src/audit.rs:174-180
Code:
```
    for key in catalog.keys() {
        let (Some(reference_message), Some(catalog_message)) =
            (reference.get(key), catalog.get(key))
        else {
            // Missing keys are rule 1's job; orphans are rule 2's.
            continue;
        };
```
What is wrong: placeholder equality (rule 3) is checked only on shared keys; an ORPHAN key's placeholders are never validated (orphans are flagged by rule 2, so a maintainer fixing the orphan by adding it to the reference then gets the check — the gap is one step of indirection only). Design choice, acceptable; recorded for completeness.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CatalogIssue::Display::fmt | 62-106 | REAL | — | Actionable one-line diagnostics per doc 07 §3.5; includes rule # and doc ref. |
| Coverage::percent | 118-126 | REAL | — | Zero-division guarded (reference_total==0 → 0.0). |
| audit_missing_keys | 128-139 | REAL | — | Reference keys minus catalog — correct direction. |
| catalog_coverage | 141-151 | REAL | — | Count of covered reference keys. |
| audit_orphan_keys | 153-166 | REAL | — | Catalog keys minus reference. |
| audit_placeholders | 168-193 | REAL | S4 | F-004 shared-keys-only. |
| format_placeholder_set | 195-204 | REAL | — | Sorted (BTreeMap) stable rendering. |
| audit_references | 206-218 | REAL | — | Filter refs not in reference. |
| audit_plural_conformance | 220-259 | REAL | S4 | F-003 lenient fallback for unknown languages. |
| plural_conformance_detail | 261-294 | REAL | — | Missing + (strict) unexpected categories; grammar of message ("categor(y|ies)") correct. |
| audit_catalogs | 296-303 | REAL | — | Thin wrapper, refs=&[]. |
| audit_catalogs_with_refs | 305-341 | REAL | — | Load failures first, ReferenceMissing early-return, reference gets rules 4+5, others get 1/2/3/5. Correct composition. |
| catalog_error_issue | 343-350 | REAL | S3 | F-002 dead API. |
| tests (6) | 352-514 | REAL | — | Genuine: exact issue sets, doc-07 example `{percent, number, percent}` vs `{percent}` (447-474), coverage math. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rule1..rule4 unit tests (audit.rs:380-513) | YES | Exact expected issue lists/matches on crafted catalogs; negative control `a_perfect_catalog_produces_no_issues`. |
| audit_proof_matrix.rs (14 tests) | YES | Each rule's violation fixture asserts the exact CatalogIssue + Display text; `locales-good` negative control; combined `audit_catalogs` output asserted to contain the same issues (lines 91, 144, 162, 193, 271, 305). |
| seed_integrity.rs (3 tests) | YES for the tree | Real `locales/` dir + real product.toml; would fail on any catalog drift (rule 1-5) or unresolved product msgid. Does NOT cover schemas/settings.toml (F-001). |
| Executed? | Baseline yes (exit_code=0); own run NOT re-executed (sandbox linker fault on theme-tokens attempt — same class as baseline attempt1-diskpoisoned). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Run by `cargo xtask l10n-audit`" (5 CI rules) | docs/07-config-driven-system.md:678 | FALSE | No such subcommand (tools/xtask/src/cmd/mod.rs:74-191 table); only tests run the audits. |
| Gate step 6 `cargo xtask l10n-audit (C3)` | docs/07-config-driven-system.md:846 | FALSE | Same proof; scripts/full_gate.sh baseline (full_gate.txt) contains no l10n step. |
| Rule 4 covers "any config file" / schemas/settings.toml | docs/07-config-driven-system.md:683, :834 | SHALLOW | Only product.toml is scanned (seed_integrity.rs:39); 121/121 settings msgids unresolved vs en-US.toml (python3). |
| 5 rules implemented as library functions | audit.rs:1-14 (self) | REAL | Verified per-function above + proof-matrix tests. |

## 7. Dead / duplicated / unreachable code
`catalog_error_issue` (F-002). The whole crate is unreachable from shipped binaries (no dependents) — see header.

## 8. Dependencies & flags
Crate deps: key/locale/message/catalog/plural/refs (internal) + toml/thiserror (refs.rs). No feature gates. No external consumers.

## 9. Open questions for the next agent
- Card 1.E2 (`xtask l10n-audit`) status: open forever or dropped? If dropped, doc 07 §6.4/§9 must be rewritten; if open, F-001 becomes S2 the day a second locale lands.
- Does any doc claim the desktop app renders window titles via this crate's fallback chain? (apps/desktop/src/window_config.rs:156 only stores the key as a string; audit of that file belongs elsewhere.)
