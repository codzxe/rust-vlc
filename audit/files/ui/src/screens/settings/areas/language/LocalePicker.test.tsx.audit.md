# AUDIT: ui/src/screens/settings/areas/language/LocalePicker.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 77 / 4 it() tests, 1 describe, 2 helpers |
| File verdict | REAL (4/4 pass in baseline; single-locale limitation honestly disclosed) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 150 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Honest closed-dropdown test: trigger is an aria-haspopup listbox showing the current locale,
opening offers EXACTLY the localization crate's real locales (asserted `AVAILABLE_LOCALES`
== ["en-US"] — the one-locale truth, not a fabricated list), the global.language binding is
probed through a real `useSettings` consumer, and `localeLabel` pins both the catalog-backed
label and the raw-tag fallback for unshipped locales (fr-FR -> "fr-FR", no invented copy).
The single-locale limitation on the select-path proof is disclosed in the test itself.

## 2. What actually works (verified)
- Baseline: ui-test.txt line 150 `✓ .../LocalePicker.test.tsx (4 tests) 158ms`; count 4 == 4.
- Read-verified: `expect(AVAILABLE_LOCALES).toEqual(["en-US"]);` (the honest one-locale
  truth) and `expect(localeLabel("fr-FR", m)).toBe("fr-FR"); // no catalog copy → raw tag`.

## 3. Findings
F-001 [S4] The binding test can only re-select the sole available option (disclosed
in-comment: "Single-locale world ..."); a real value CHANGE is never exercised — draft-dirty
path for a different locale is unpinned until a second locale ships. Confidence: HIGH — read.
F-002 [S4] `latest = String(value(...))` writes during render (Probe) — works but relies on
subscription-less snapshot timing; `await`-less async test declares `async` without awaiting
anything (lint-passing but vacuous async). Cosmetic. Confidence: HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| mount | 17-22 | REAL | real provider with memory persistence |
| 4 tests | 24-77 | REAL | closed trigger; exact real locales; binding probe; label fallback |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| offers exactly the real available locales | YES | count == AVAILABLE_LOCALES.length AND the literal array |
| labels the reference locale through the i18n catalog | YES | both branches (catalog + raw fallback) asserted |
| binds to the global.language setting key | PARTIAL | re-select only (disclosed, F-001) |

## 6-9. Doc claims / dead code / deps / open questions
Baseline 4 == 4. None / vitest + generated schema + i18n / none.
