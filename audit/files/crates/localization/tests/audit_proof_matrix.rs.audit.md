# AUDIT: crates/localization/tests/audit_proof_matrix.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2025-06-14 |
| Lines / functions | 368 / 14 tests + 2 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — the `localization` crate has ZERO production consumers: no app/crate Cargo.toml depends on it (only its own), and lumen-cli/desktop/ui never import it. Enforcement is `cargo test`-only (this file + tests/seed_integrity.rs over the real `locales/` tree). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p localization --test audit_proof_matrix` -> 14 passed / 0 failed (0.00s); rg over tools/xtask (no l10n command exists); read of crates/localization/src/audit.rs (514 LOC, genuine implementations); sed quote verification. |

## 1. One-paragraph verdict
A genuinely hostile test matrix: every one of the five doc 07 §6.4 rules is proven in both directions — a violating fixture must produce an exact, ordered issue list with the exact key/file/field, and negative controls (`locales-good`, `refs/resolved.toml`, unknown-language `xx`) must produce zero. The library functions it exercises are real algorithms (set-difference for missing/orphan keys, placeholder-set equality, CLDR arm-set comparison), not canned outputs. 14/14 pass. The two problems are around the file, not in it: the doc-07-promised `cargo xtask l10n-audit` CI command does not exist (enforcement rides only on cargo test), and the entire crate is an island with no shipped consumer.

## 2. What actually works (verified)
— verified by `cargo test -p localization --test audit_proof_matrix` -> `test result: ok. 14 passed; 0 failed`, plus read of audit.rs:126-300 confirming `audit_missing_keys`/`audit_orphan_keys`/`audit_placeholders`/`audit_references`/`audit_plural_conformance` are real set/logic operations over loaded catalogs (no fixture-baked returns).

## 3. Findings
F-001 [S3]
Where: crates/localization/tests/audit_proof_matrix.rs:16-18 vs docs/07-config-driven-system.md:678 vs tools/xtask/src/cmd/ (directory listing)
Code (test, honest future tense):
```
//! through the combined `audit_catalogs`/`audit_catalogs_with_refs` entry
//! points that CI (card 1.E2 `cargo xtask l10n-audit`) will call.
```
vs docs/07-config-driven-system.md:678 (present tense):
```
Run by `cargo xtask l10n-audit` (backed by `crates/localization::audit_catalogs`, doc 01 §2.3):
```
What is wrong: `cargo xtask l10n-audit` does not exist — tools/xtask/src/cmd/ contains 19 command files, none l10n; no xtask source mentions `audit_catalogs`. The doc's §6.4 "Compile-time tests (CI)" section asserts a runner that was never written. Why it matters: the five rules ARE enforced, but only when someone runs `cargo test -p localization`; no CI gate, no xtask lane, and no shipped binary pulls the crate in — the localization plane is enforcement-by-convention. What correct looks like: implement the xtask command (a ~20-line wrapper over `audit_catalogs`) or fix doc 07 §6.4 to name the real runner (`cargo test -p localization`).
Confidence: HIGH — Verification: read (xtask cmd dir listed; rg `l10n` in tools/xtask -> only lint_literals.rs prose).

F-002 [S4]
Where: crates/localization/tests/audit_proof_matrix.rs:321-331
Code:
```
    let empty = std::env::temp_dir().join(format!("lumen-l10n-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    let _ = std::fs::create_dir(&empty);
    let issues = audit_catalogs(&empty);
    let _ = std::fs::remove_dir_all(&empty);
    assert_eq!(issues.len(), 1);
```
What is wrong: all three fs calls swallow errors; if `create_dir` fails (read-only temp, race), `audit_catalogs` on a nonexistent path returns `CatalogUnreadable` (proven by `missing_locales_directory_is_reported`), so the assert dies with a misleading "expected 1, got N / wrong variant" instead of "could not create temp dir"; a panic before cleanup also leaks the dir. Blast radius: one test's diagnostics only. Confidence: HIGH — Verification: read + ran (passes today).

## 4. Per-function table (test file; >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixtures / load_pair | 30-62 | REAL | — | Fails loudly on malformed fixtures |
| rule1_missing_keys_are_flagged_exactly | 68-99 | REAL | — | Exact ordered key list + coverage math 4/6≈66.67 (float-checked) + combined entry |
| rule2_duplicate_keys_fail_at_parse_level | 105-140 | REAL | — | Proves TOML parse-level duplicate detection, reference still loads |
| rule2_duplicate_keys_surface_through_the_combined_audit | 142-151 | REAL | — | CatalogUnreadable + "duplicate key" substring |
| rule2_orphan_keys_are_flagged_exactly | 153-166 | REAL | — | Exact file+key match |
| rule3_placeholder_mismatch_is_flagged_exactly | 172-200 | REAL | — | Asserts expected/actual contents + Display text |
| rule4_unresolved_references_are_flagged_exactly | 206-235 | REAL | — | 2 refs extracted, 1 flagged, negative control clean |
| rule4_file_extraction_helper_reports_missing_files_as_none | 237-247 | REAL | — | Ok(None) contract for absent files |
| rule5_missing_plural_categories_are_flagged | 253-278 | REAL | — | ru: missing few/many exact |
| rule5_unused_plural_categories_are_flagged | 280-296 | REAL | — | ja: `one` not used by language |
| rule5_unknown_language_enforces_only_the_parse_level_other | 298-308 | REAL | — | xx: other-only, extras tolerated |
| a_conformant_tree_produces_zero_issues | 314-318 | REAL | — | Negative control |
| missing_reference_catalog_is_reported | 320-331 | REAL | S4 | F-002: swallowed fs errors |
| missing_locales_directory_is_reported | 333-342 | REAL | — | CatalogUnreadable on missing dir |
| fixture_reference_catalogs_are_identical_across_directories | 349-368 | REAL | — | Anti-drift: 8 dirs share byte-identical en-US.toml + parses |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 14 | YES | Exact issue-count + variant + field assertions on both directions; e.g. deleting the orphan-key filter fails rule2_orphan (expects exactly 1 OrphanKey, gets 0); making audit_plural_conformance lenient fails rule5 (expects 1 issue); a broken coverage percent() fails the float check in rule1 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Run by `cargo xtask l10n-audit`" | docs/07-config-driven-system.md:678 | FALSE (command never written) | tools/xtask/src/cmd/ listing; F-001 |
| "the five doc 07 §6.4 CI audit rules enforced by fixtures" | audit_proof_matrix.rs:1-2 | REAL (rules) / aspirational (CI) | 14/14 pass; CI runner absent |
| "{percent, number, percent} in en-US and {percent} in de is a CI failure" | doc 07 §6.4 rule 3 | REAL | rule3 test asserts exactly this fixture pair |

## 7. Dead / duplicated / unreachable code
- The entire `localization` crate is unreachable from shipped binaries (no dependents). This test file is its de-facto production consumer (via cargo test enforcement of `locales/`).

## 8. Dependencies & flags
Test-only deps: the crate itself. No feature gates, no #[ignore].

## 9. Open questions for the next agent
- Who owns card 1.E2: the xtask l10n-audit wiring is the missing piece; `xtask feature-audit` FAILURE (baseline) suggests the xtask lane has drift issues generally.
- locales/ ships only en-US.toml — the rule-1/2/3/5 machinery has no real non-reference catalog to guard yet (fixtures only).
