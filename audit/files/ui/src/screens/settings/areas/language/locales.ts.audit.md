# AUDIT: ui/src/screens/settings/areas/language/locales.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A05) |
| Date | 2026-11-24 |
| Lines / functions | 30 / 1 export fn (localeLabel) + 2 consts (AVAILABLE_LOCALES, LOCALE_LABEL_MSGIDS) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — locales.ts → LocalePicker.tsx:8,35,44 → specialWidgets.ts:60 `"global.language": LocalePicker` → SettingField.tsx:9 → generated Global area screen → SettingsScreen → App.tsx:108 `/settings/:area` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | ls locales/ -> en-US.toml only; sed -n '289,315p' crates/localization/src/catalog.rs -> scan verified; rg REFERENCE_LOCALE / settings.locale.en-us / consumers -> all hits cited; baseline ui-test.txt:150 |

## 1. One-paragraph verdict
A one-locale data module whose every claim is verifiably true: `locales/` contains exactly one catalog (en-US.toml), the localization crate really scans that directory for `*.toml` (catalog.rs:289-315), the reference locale is "en-US" (catalog.rs:23), the label msgid resolves centrally to "English" (messages.ts:1785), and the w26.k13 cleanup note is accurate — no test-only REFERENCE_LOCALE export exists anymore. Honest single-source-of-truth design with the drift risk disclosed in both this header and the schema's [VERIFY] marker.

## 2. What actually works (verified)
— verified by `ls locales/` -> `en-US.toml` (one file = one catalog, matching AVAILABLE_LOCALES).
— verified by baseline ui-test.txt:150 `✓ LocalePicker.test.tsx (4 tests)`; areas.test.tsx:256-258 asserts every AVAILABLE_LOCALES entry is backed — the picker renders through this data.

## 3. Findings
None. Checked and clean: (a) list-vs-directory drift — hand list matches the actual directory scan exactly (verified today; growth path documented lines 8-9); (b) `localeLabel` fallback — returns the raw tag when no msgid (line 29), total function, no undefined path; (c) phantom export claimed removed by w26.k13 — confirmed absent; (d) key existence — "settings.locale.en-us" in MessageKey union (messages.ts:306) and catalog (:1785); (e) stringly-typed parallel structures (AVAILABLE_LOCALES + LOCALE_LABEL_MSGIDS keyed by plain string) — acceptable at n=1, fallback documented.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AVAILABLE_LOCALES | 14 | REAL | — | ["en-US"], matches locales/ dir exactly |
| LOCALE_LABEL_MSGIDS | 22-24 | REAL | — | one entry, key resolves centrally |
| localeLabel | 27-30 | REAL | — | total: msgid ? m(msgid) : tag |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| areas.test.tsx:256-258 (every locale has backing) | YES | iterates AVAILABLE_LOCALES and asserts catalog backing — adding a bogus tag fails it |
| LocalePicker.test.tsx (4 tests, baseline :150) | YES | renders the picker against this data source |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the localization crate … resolves catalogs by scanning the repo `locales/` directory" | locales.ts:3-4 | REAL | catalog.rs:289-315 read_dir + `.toml` filter (sed-verified) |
| "today that directory contains exactly one catalog — … en-US" | locales.ts:5-6 | REAL | `ls locales/` -> en-US.toml; catalog.rs:23 REFERENCE_LOCALE = "en-US" |
| "the test-only REFERENCE_LOCALE export is gone" | locales.ts:16-19 | REAL | file read: no such export |

## 7. Dead / duplicated / unreachable code
None. Both exports consumed (LocalePicker.tsx:8 production; areas.test.tsx:19 test).

## 8. Dependencies & flags
i18n/messages (MessageKey type only). No feature gates, no runtime deps.

## 9. Open questions for the next agent
When a second locale lands: who generates AVAILABLE_LOCALES? The schema's `[VERIFY] dynamic locales/ listing` (settings.toml:67) is still open — the hand list is the interim answer, and two structures (list + msgid map) will need the same growth discipline.
