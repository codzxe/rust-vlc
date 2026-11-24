# AUDIT: crates/localization/src/refs.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 221 / 6 (refs_parse_error, extract_msgid_refs, extract_msgid_refs_from_file, walk_table, walk_value, Display) + 4 tests |
| File verdict | REAL (correct, well-tested extractor) — UNREACHABLE from shipped entry points (crate has zero dependents) |
| Reachable from shipped entry point? | NO. Callers: crates/localization/src/audit.rs:209 (audit_references input type), tests/seed_integrity.rs:37 (product.toml only), tests/audit_proof_matrix.rs:210,231,242,244 (fixtures). No shipped binary links this crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg callers (exit 0, list above); python3 rule-4 simulation over schemas/settings.toml (121 refs, 121 unresolved vs en-US.toml); Read tool for byte-safe quotes. |

## 1. One-paragraph verdict
A real, conservative msgid-reference extractor: TOML parse with positioned errors, recursive walk over tables and arrays-of-tables, and a field-name allowlist (`label_key`, `description_key`, `title_idle_key`, `description`) whose values must additionally pass the doc 07 §6.1 key grammar before being counted. Tests are genuine (exact extraction sets, prose-ignoring negative, typed parse errors). The defect is scope-of-use, not implementation: in the whole tree it is pointed at exactly ONE real file (product.toml, 2 refs), while doc 07 §6.4 rule 4 names six source families — the 121 msgid references in schemas/settings.toml are never extracted by anything and none of them resolve in the reference catalog.

## 2. What actually works (verified)
- Baseline: localization package 45 lib tests pass (incl. refs.rs's 4), audit_proof_matrix 14 pass — exit_code=0 (cargo-test-workspace.txt:2879-2977).
- Extraction semantics re-implemented in python3 over schemas/settings.toml (same field allowlist + grammar): 121 refs found — proving the extractor WOULD find them if anything called it with that path; 121/121 missing from locales/en-US.toml.

## 3. Findings
F-001 [S3]
Where: crates/localization/src/refs.rs:5-9 (documented scope) vs the sole real caller, tests/seed_integrity.rs:37
Code:
```
//! The reference sources named by doc 07 §6.4 are `product.toml`,
//! `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`,
//! `themes/*` (and doc 05's menu/UI-action tables, which materialize as
//! `layouts/menus.toml`).
```
What is wrong: of the named sources, only product.toml exists AND is scanned; schemas/settings.toml exists, is named, and is NEVER passed to `extract_msgid_refs_from_file` by any code or test (rg proof above). Its 121 msgid-shaped `description` values are all absent from locales/en-US.toml (python3). layouts/, profiles/ do not exist in the repo (ls: only keymaps/, schemas/), keymaps/default.toml has no msgid fields (rg `label_key` → product.toml only). So the extractor's documented input universe is 1/6 wired, and the one unwired-but-present source would fail rule 4 wholesale. Same root cause as audit.rs F-001 (card 1.E2 never landed).
Confidence: HIGH — Verification: read + rg + python3.

F-002 [S4]
Where: crates/localization/src/refs.rs:126-135
Code:
```
        toml::Value::String(text) => {
            if MSGID_FIELDS.contains(&field)
                && let Ok(key) = MessageKey::new(text)
            {
                refs.push(MsgidRef {
```
What is wrong: heuristic edge: any `description` string that happens to look like a key (e.g. a theme note "a.b") becomes a reference and would then be reported unresolved by rule 4. Documented as deliberate ("Extraction is conservative", lines 17-20) and today's real configs contain no such false positives (python3 scan: all 121 extracted settings descriptions ARE intended msgids). Inverse risk — a real msgid in a non-allowlisted field (e.g. `label`, `title`) is silently ignored — is also accepted by design.
Confidence: HIGH — Verification: read + python3.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MSGID_FIELDS / MsgidRef / Display | 30-52 | REAL | — | Field table matches doc 07 §3.2/§5.2/§7.2 (self-test at 210-220). |
| RefError / refs_parse_error | 55-82 | REAL | — | Line/col from error span, (1,1) fallback. |
| extract_msgid_refs | 85-90 | REAL | — | TOML parse → walk. |
| extract_msgid_refs_from_file | 96-109 | REAL | — | Missing file = Ok(None), honestly documented ("documented honesty, not a skip", 93-95). Real I/O errors typed. |
| walk_table / walk_value | 111-142 | REAL | S4 | F-002 heuristic; arrays-of-tables keep field context (122-124) — correct. |
| tests (4) | 148-221 | REAL | — | Exact 4-ref set with documented BTreeMap ordering caveat; prose/negative; typed parse error; field-table change-detector. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| extracts_known_msgid_fields | YES | Exact string set of all four refs incl. a [[menu.leaf]] array-of-tables entry. |
| ignores_non_msgid_values_and_prose | YES | Empty-set assertion over prose/technical/nested-invalid cases. |
| parse_failures_are_typed_errors | YES (weak) | Matches the variant only; no line/col assertion. |
| msgid_field_table_matches_doc07 | Change-detector | Re-asserts the constant; fails only if someone edits MSGID_FIELDS — its purpose. |
| Executed? | Baseline yes (45 lib tests pass). Not re-run locally (sandbox linker fault — log). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Reference sources = 6 families incl. schemas/settings.toml | refs.rs:5-9; docs/07 §6.4 rule 4 | SHALLOW | Extractor handles them; nothing scans them (F-001). |
| "Extraction is conservative: only values that syntactically ARE message keys … are treated as msgid references" | refs.rs:17-20 | REAL | Verified by grammar gate + negative test. |

## 7. Dead / duplicated / unreachable code
The whole module is reachable only from the crate's own audit entry + tests (no external dependents). No internal dead code.

## 8. Dependencies & flags
toml + thiserror + internal key/line_col. `toml::Table` is BTreeMap-backed (no preserve_order feature) — the ordering caveat in the first test's comment is accurate.

## 9. Open questions for the next agent
- If card 1.E2 lands, should keymaps/* gain msgid fields, or is the doc list aspirational? (keymaps/default.toml currently has none — rg.)
