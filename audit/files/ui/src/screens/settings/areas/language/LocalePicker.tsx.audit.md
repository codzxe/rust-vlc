# AUDIT: ui/src/screens/settings/areas/language/LocalePicker.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 52 / 1 (LocalePicker) + hook factory use |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — settings language area (areas/index registry) -> SettingsScreen; binds the settings draft via useSettings |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read locales.ts; ls ../locales -> exactly en-US.toml (the "real locales only" claim verified) |

## 1. One-paragraph verdict
The Interface-language dropdown, correctly pessimistic: options are the
INTERSECTION of the schema enumeration and AVAILABLE_LOCALES, and
AVAILABLE_LOCALES is exactly ["en-US"] because the repo locales/ directory
contains exactly one catalog (verified: en-US.toml only). The picker cannot
offer a locale that does not ship. Value binds the real settings draft
(restart badge comes from the schema flag). No findings.

## 2. What actually works (verified)
- `ls locales/` -> en-US.toml (single file) == AVAILABLE_LOCALES = ["en-US"] — the "real locales only, never invented" claim holds today.
- Intersection filter (line 34-35): a schema enum value without a catalog is not offered; a catalog without a schema binding cannot be set.
- Label fallback: raw tag shown when copy missing (localeLabel), never an invented name.

## 3. Findings
None. Checked: dead options (none — intersection), i18n seam (makeAreaMsg
with lane keys), a11y (Dropdown carries combobox/listbox/roving tabindex per
its own contract — component audited elsewhere).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| usePickerMsg | 26 | REAL | makeAreaMsg dedup |
| LocalePicker | 28-52 | REAL | intersection + draft binding |

## 5. Test honesty
Not read (LocalePicker tests belong to the language-area suite if assigned);
the intersection logic is trivially reviewable and data-pinned.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Options stay the INTERSECTION of the schema enumeration and the localization crate's real available locales" | LocalePicker.tsx:18-20 | REAL | filter at 34-35 + locales.ts single entry verified |
| "today that directory contains exactly one catalog — the reference locale en-US" | locales.ts:5-7 | REAL | ls locales/ -> en-US.toml |

## 7. Dead / duplicated / unreachable code
None (the test-only REFERENCE_LOCALE export was already pruned per
locales.ts comment — verified absent).

## 8. Dependencies & flags
Dropdown, SettingsProvider, shared makeAreaMsg, locales data. No gates.

## 9. Open questions for the next agent
- The schema's global.language enumeration vs the one shipped locale: if the
  enum promises many languages, the UI silently shows one option — check the
  enum in schemas/settings.toml for honesty of the schema itself (engine
  lane).
