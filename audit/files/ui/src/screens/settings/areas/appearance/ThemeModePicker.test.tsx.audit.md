# AUDIT: ui/src/screens/settings/areas/appearance/ThemeModePicker.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 74 / 4 tests in 1 describe |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for the shipped appearance ThemeModePicker; runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "ThemeModePicker.test" audit/runs/baseline/ui-test.txt` → `153: ✓ src/screens/settings/areas/appearance/ThemeModePicker.test.tsx (4 tests) 147ms`; full file read |

## 1. One-paragraph verdict
Honest segmented-control tests over the real provider chain (ThemeProvider → SettingsProvider with memory persistence → real schema entry): exactly three radio cells in a radiogroup with the segmented grammar class, the dropped hint paragraph pinned by absence, cell clicks bound to the settings draft with live theme preview on <html> (light) plus auto/dark round-trips, and arrow-key roving pinned to the schema order. The store values are read through a real useSettings probe.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L153 `✓ ... (4 tests) 147ms` (inside 1705 passed).
- Draft binding verified through the provider (L39-64): latest value goes "light" → "auto" → "dark"; documentElement dataset.theme checked after the light click.

## 3. Findings
None. Zero-findings checklist: tautology scan — all expects bind to rendered roles/classes/draft values/html attributes of the real component. Mock-of-subject scan — none; memorySettingsPersistence is the real persistence seam for tests, Probe reads the real store. Skip scan — no it.skip; baseline 4 passed. Name-vs-body scan — verified. Mutation reasoning: wrong cell count fails L31; resurrected hint fails L36; unbound clicks fail L54-63; broken roving fails L69-72. Note (cosmetic, no finding): L57 `await (() => expect(...))()` is an IIFE-wrapped synchronous assertion — it executes fully; the await is decorative.

## 4. Per-function table (<100 LOC; still listed)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| three cells radiogroup | 24-32 | REAL | — | exact count + grammar class |
| dropped hint | 34-37 | REAL | — | absence pin |
| click binding + live preview | 39-64 | REAL | — | draft values + html theme attr |
| roving arrows | 66-73 | REAL | — | schema-order landing |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 | YES | role/attr/store-value assertions; verified green baseline L153 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the Theme row control is the frame's 3-cell SEGMENTED control ... not a stacked radio list" | comment L9-12 | REAL | radiogroup with exactly 3 radios + lumen-segmented class |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@testing-library/react, vitest; real ThemeModePicker/SettingsProvider/persistence/schema/ThemeProvider.

## 9. Open questions for the next agent
- None.
