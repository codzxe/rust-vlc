# AUDIT: crates/localization/src/refs.rs
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-21 |
| Lines / functions | 221 / 12 (incl. 4 in-file tests) |
| File verdict | UNREACHABLE (mechanism real + tested; every declared consumer is either absent, misnamed, or a test) |
| Reachable from shipped entry point? | NO — proof below |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 1 |
| Commands run | `rg -n "localization" --glob '**/Cargo.toml'` (zero dependents); `rg -ln "label_key" --glob '!audit/**'` (no shipped config file uses it); `rg -c "label-msgid" keymaps/default.toml` (151); `rg -n "hotkeys\." locales/en-US.toml` (0) and `ui/src/i18n/messages.ts` (291); `rg -o 'description = "…"' schemas/settings.toml` (121) cross-checked against messages.ts (11 engine/extensions keys absent — deliberate non-UI); read tests/seed_integrity.rs, src/audit.rs:209-332, ui/scripts/gen-hotkeys-data.mjs; `sed -n` on all quoted ranges |

## 1. One-paragraph verdict
`refs.rs` is the input side of catalog-audit rule 4 ("every msgid referenced from `product.toml`, `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`, `themes/*` resolves to the reference catalog"). The walker is small and its tests are exact — and it is blind to the shipped tree in three compounding ways. (1) Its field vocabulary contains `label_key`, a name that appears in **zero** shipped config files (the `layouts/` directory it comes from does not exist), while the one large msgid-carrying config that does ship, `keymaps/default.toml`, names the field `label-msgid` — 151 references the extractor structurally cannot see. (2) Values that fail the key grammar are silently dropped (`let Ok(key) = MessageKey::new(text)`), so a typo'd msgid — the exact defect rule 4 exists to catch — never enters the reference set; the only compensating gate (settings-schema D4) checks uniqueness, not validity or resolvability. (3) The sole shipped-tree invocation of the extractor scans `product.toml` alone (2 refs, both resolve); the 121 `settings.*` description msgids and 151 `hotkeys.*` label msgids are never scanned, and none of them exist in the 16-key reference catalog `locales/en-US.toml`. Like the rest of the crate, the code has no production caller — `cargo xtask l10n-audit` is un-landed card 1.E2 — so rule 4 is a paper rule enforced on 2 of ~274 declared references.

## 2. What actually works (verified)
— verified by `sed -n` full trace + baseline #3 (`audit/runs/baseline/test-workspace.log.txt`, 1681 passed): the TOML walk extracts `description_key`/`title_idle_key`/`label_key`/`description` strings from tables, arrays, and arrays-of-tables; parse failures become positioned typed errors; the 4 in-file tests assert exact extraction output (sorted BTreeMap order documented honestly at refs.rs:167-171).
— verified by direct cross-check (`rg`): product.toml's 2 refs (`product.description`, `window.title.idle`) exist in `locales/en-US.toml:70,75` — the one slice of rule 4 that actually runs, passes.

## 3. Findings

### F-001 [S2]
Where: crates/localization/src/refs.rs:28-35
Code:
```
/// Field names whose string values are msgid references (doc 07 §3.2,
/// §5.2, §7.2). Values that do not match the key grammar are ignored.
pub const MSGID_FIELDS: &[&str] = &[
    "label_key",
    "description_key",
    "title_idle_key",
    "description",
];
```
What is wrong: The vocabulary does not match any shipped config that carries label msgids. `rg -ln "label_key"` over the non-audit tree hits only this crate's docs/tests/fixtures and UI TypeScript — no TOML config file. The real keymap file uses `label-msgid`: `keymaps/default.toml:50` — `open-file = { key = "Ctrl+O", label-msgid = "hotkeys.open-file", … }` — 151 occurrences (150 binding rows + 1 comment). `layouts/`, the documented home of `label_key` (doc 07 §7.2, and this file's header line 7 "layouts/menus.toml"), **does not exist** (`ls layouts/` → No such file or directory). The naming mismatch is self-acknowledged in the keymap header itself (keymaps/default.toml:29-32 "Reconcile the two before the `schemas/keymap.schema.json` validator lands").
Why it matters: Rule 4's declared coverage of `keymaps/*` is structurally impossible with this vocabulary — even if someone wired `extract_msgid_refs_from_file` over `keymaps/default.toml` today, it would return **zero** references and "pass". The audit tool would certify a file it never read.
What correct looks like: either the extractor grows `label-msgid` (hyphenated, matching the shipped 0.B3/D-003 format) or the keymap file migrates to the doc-05 format — before the `l10n-audit` card lands, plus a test that runs rule 4 over the real `keymaps/` directory.
Confidence: HIGH — Verification: read + rg counts + directory listing.

### F-002 [S2]
Where: crates/localization/src/refs.rs:126-135
Code:
```
        toml::Value::String(text) => {
            if MSGID_FIELDS.contains(&field)
                && let Ok(key) = MessageKey::new(text)
            {
                refs.push(MsgidRef {
```
What is wrong: A msgid-field value that fails the doc-07 §6.1 grammar (typo: underscore, uppercase, 6th segment, prose) is silently discarded — the `Err` arm of `MessageKey::new` is an invisible skip. The module header (refs.rs:17-20) sells this as "conservative" filtering of prose `description` fields, but it also filters *broken* msgids: the most likely reference defect is the one the rule can never report. No compensating gate exists: settings-schema's D4 checks only that description msgids are *unique* (`d4_descriptions_unique`, crates/settings-schema/src/validate.rs:237-256), not that they are valid or resolvable.
Why it matters: Rule 4's failure mode — a settings row or keymap row whose label msgid nobody can render — is exactly the class Wave 8 found live (`audio.set-eq` to a nonexistent backend). Under this extractor a `description = "settings.playback.seek_step"` (underscore typo) passes every audit silently.
What correct looks like: extract the raw string whenever the *field name* matches, then report two distinct issues: "invalid msgid grammar" (value fails `MessageKey::new`) and "unresolved msgid" (key absent from the reference catalog); restrict the prose-tolerance to sources whose `description` is contractual prose (themes), which the extractor knows per-source.
Confidence: HIGH — Verification: read + settings-schema D4 traced.

### F-003 [S3]
Where: crates/localization/src/refs.rs:1-9 (header) + the sole shipped-tree caller, crates/localization/tests/seed_integrity.rs:37-38
Code:
```
//! The reference sources named by doc 07 §6.4 are `product.toml`,
//! `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`,
//! `themes/*` (and doc 05's menu/UI-action tables, which materialize as
//! `layouts/menus.toml`).
```
```
    let refs = extract_msgid_refs_from_file(&repo("product.toml"))?.unwrap_or_default();
    assert_eq!(refs.len(), 2, "product.toml msgid refs: {refs:?}");
```
What is wrong: The header names six source groups; the enforcement (the crate's own "negative control over the repository itself", since `cargo xtask l10n-audit` is un-landed card 1.E2) scans exactly one: `product.toml`, asserting the count is 2. Meanwhile the tree carries 121 `description = "settings.*"` msgids in `schemas/settings.toml` (rg -c) and 151 `label-msgid` refs in `keymaps/default.toml` — none of which resolve in the reference catalog: `locales/en-US.toml` contains exactly 16 keys (`product.description`, `window.title.idle`, 13 `xtask.*`, `ci.job.stub-suffix` — rg -n listing, lines 70-97) and zero `settings.*`/`hotkeys.*` keys. The real resolver for those msgids is the TypeScript catalog (`ui/src/i18n/messages.ts`, 291 `hotkeys.*` entries) which this Rust rule never sees.
Why it matters: The claim "every msgid referenced from … resolves" (doc 07 §6.4 rule 4, lib.rs:36-40) is enforced for 2 of ~274 shipped references. The reference catalog itself is a 16-key stub, so any honest widening of the scan would instantly fail the build — which is presumably why it was not widened.
What correct looks like: either the reference catalog grows to be the real union catalog (generated from or checked against `ui/src/i18n/messages.ts`) and the seed test scans all six source groups, or doc 07 §6.4 is rewritten to state the actual Phase-1 scope (product.toml only).
Confidence: HIGH — Verification: read + counts by rg + `ls layouts/ profiles/` (absent).

### F-004 [S4]
Where: crates/localization/src/refs.rs:111-125
Code:
```
fn walk_table(file: &str, table: &toml::Table, refs: &mut Vec<MsgidRef>) {
    for (field, value) in table {
        walk_value(file, field, value, refs);
    }
}
```
What is wrong: Matching is purely by field name at any depth in any scanned file — there is no per-source field contract. A prose `description` (or a future `label_key`) anywhere in a scanned file whose text happens to parse as a key becomes a phantom reference (false positive), while the same mechanism guarantees the false negatives of F-002. Today no shipped file trips it (product.toml carries no bare `description`; rg verified), so blast radius is latent.
Why it matters: One mechanism cannot simultaneously be "ignore prose" and "catch broken msgids" — the two goals need different information (source contract) than the extractor possesses.
What correct looks like: per-source field lists (settings.toml: `description` is always a msgid; themes: `description` is always prose) supplied by the caller, mirroring how `SemanticInputs` gates product-config's checks.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MSGID_FIELDS` | 28-35 | REAL but mismatched | S2 (F-001) | `label_key` matches zero shipped configs; `label-msgid` (151 uses) missing; `description_key`/`title_idle_key` match product.toml |
| `MsgidRef` (struct) + `Display` | 37-52 | REAL | — | Plain data; Display `file:field -> key` used in test assertions |
| `RefError` (2 variants) | 54-67 | REAL | — | thiserror; `Parse` carries line/column (position duplicated from toml's own message — cosmetic); `Io` keeps context+detail |
| `refs_parse_error` | 69-82 | REAL | — | Span→(line,col) via `line_col` (char-indexed, verified in lib.rs tests); falls back (1,1) |
| `extract_msgid_refs` | 84-90 | REAL | — | Parse → walk; no panics; allocates Vec |
| `extract_msgid_refs_from_file` | 92-109 | REAL | — | NotFound → `Ok(None)` — documented honesty (Phase-1 absent files); other IO errors typed; a directory at `path` surfaces as Io error, not silent success |
| `walk_table` | 111-115 | REAL | S4 (F-004) | Depth-blind recursion |
| `walk_value` | 117-142 | REAL | S2 (F-002) | Arrays keep field context (correct for `[[setting]]`); non-string TOML values skipped; the `let Ok(key) = …` silent drop is F-002 |
| tests: `extracts_known_msgid_fields` | 148-183 | REAL | — | Pins exact sorted output incl. array-of-tables and nested-table cases; documents BTreeMap ordering honestly |
| tests: `ignores_non_msgid_values_and_prose` | 185-197 | REAL | — | 4 negative cases (prose, wrong field, non-string, invalid value) — but this test *celebrates* the F-002 behavior |
| tests: `parse_failures_are_typed_errors` | 199-207 | REAL | — | matches! on Parse variant |
| tests: `msgid_field_table_matches_doc07` | 209-220 | REAL | — | Const equality — editing the vocabulary fails it (it enforces consistency with *this file's* doc, not with the shipped configs — F-001) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| extracts_known_msgid_fields | YES | Exact 4-element output vector asserted |
| ignores_non_msgid_values_and_prose | YES | Empty-refs assertion over 4 negative cases |
| parse_failures_are_typed_errors | YES | Parse-variant match on a real bad TOML string |
| msgid_field_table_matches_doc07 | YES | Array equality |
| (external) seed_integrity.rs:31-42 | YES for what it scans / NO for the rule it claims | Genuinely asserts product.toml's 2 refs resolve — but scanning 1 of 6 declared sources means it cannot fail for the 272 unscanned references (F-003) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every msgid referenced from `product.toml`, `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`, `themes/*` resolves to a reference-catalog key" | doc 07 §6.4 rule 4 / lib.rs:36-40 / refs.rs:5-6 | FALSE (as enforcement) | Only product.toml scanned (seed_integrity.rs:37); keymap field name not in MSGID_FIELDS (F-001); 121 settings + 151 keymap refs resolve in no Rust catalog |
| "doc 05's menu/UI-action tables … materialize as `layouts/menus.toml`" | refs.rs:7-8 | PAPER-ONLY | `layouts/` does not exist |
| "Extraction is conservative: only values that syntactically ARE message keys … are treated as msgid references" | refs.rs:17-20 | SHALLOW | True — and it is the defect: grammar-invalid msgids are dropped, not reported (F-002) |
| "A missing file is `Ok(None)` … documented honesty, not a skip" | refs.rs:92-94 | REAL | Code matches comment exactly |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable in production: zero workspace dependents on the crate; callers are `audit.rs` (in-crate, itself test-only-reached) and the crate's test suites.
- `MSGID_FIELDS` entry `label_key`: matches zero shipped config files (F-001).

## 8. Dependencies & flags
- `std` + `toml` + `thiserror` + crate-local `key`/`line_col`. No features. Nothing links the crate (see key.rs audit F-001) — UNREACHABLE from `lumen`/`lumen-tauri`.

## 9. Open questions for the next agent
- `cargo xtask l10n-audit` (card 1.E2): when it lands, will it scan all six sources with the corrected vocabulary? Until then rule 4 is decorative.
- `hotkeys.seek-backward-short` / `hotkeys.seek-forward-short` (keymaps/default.toml:276-277) resolve in NO catalog anywhere (not in locales/en-US.toml, not in ui/src/i18n/messages.ts) — invisible today because the UI generator strips `label-msgid` from `[platform_overrides]` rows (gen-hotkeys-data.mjs:70) and no runtime consumes overrides. Filed from the keymaps side in my next audit; cross-check with whoever owns ui/scripts/gen-hotkeys-data.mjs.
- 11 `settings.engine.*`/`settings.extensions.*` description msgids absent from the UI catalog — engine-internal settings by design? If so, why do they carry UI msgids at all?
