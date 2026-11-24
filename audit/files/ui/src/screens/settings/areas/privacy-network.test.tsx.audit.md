# AUDIT: ui/src/screens/settings/areas/privacy-network.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 160 / 8 (2 describes) |
| File verdict | REAL (baseline-verified green; staged-inert grammar + no-invention rules genuinely asserted) |
| Reachable from shipped entry point? | YES — privacy-network.tsx renders /settings/network + the privacy area's rows inside SettingsScreen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline ui-test.txt:92 |

## 1. Verdict
Covers the honest-empty/staged posture of the network area (all 9 rows staged inert with
tooltip + seam marker, 4 dimmed groups disabled, ⓘ census exactly 5/9, zero live
[data-setting] bindings, no SOCKS inventions) and the privacy frame rows (telemetry keys
remain schema-truth CLI-only, Clear-history disabled with reason, Last.fm wells verbatim,
no Browse companions). Baseline green: ui-test.txt:92 `✓ …(8 tests) 413ms`.

## 2. What actually works (verified)
- Baseline 8/8 (cite above).
- Mutation reasoning: a staged row going live fails :63 (data-setting count must be 0);
  missing dim fails :59-61; census drift fails :72-73; telemetry row reappearing on screen
  fails :129-132; invented proxy options fail :91-92; ⓘ not operable fails :78-84 (expand
  then collapse assertions).
- The schema-truth test reads generated constants directly — the ui=false relocation is
  pinned as data, not prose (:102-110).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/privacy-network.test.tsx:20-21
Code:
const uiKeys = new Set(SETTINGS_ENTRIES.map((e) => e.key));
const isScreenRow = (entry: SettingEntry): boolean => uiKeys.has(entry.key);
What is wrong: `isScreenRow` answers "is this entry's key present in SETTINGS_ENTRIES" — it
checks registry membership, not screen rendering. The name says screen-row, the body says
registry-member; a future split between the registry and actual rendering would silently
invalidate the helper's meaning while its name keeps asserting more.
What correct looks like: name it `isUiRegistryKey`.
Confidence: HIGH — Verification: read.

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| staged-inert rows | YES | 9 rows with markers + titles |
| ⓘ census + operable | YES | exact count 5 + expand/collapse |
| no live bindings/inventions | YES | counts and absence assertions |
| routing one area root | YES | querySelectorAll length 1 |
| telemetry schema truth | YES | literal default/ref/screen fields |
| privacy History rows | YES | aria-checked states + Clear disabled |
| last.fm Network section | YES | switch states + empty wells |
| no Browse on url rows | YES | absence + data-kind text |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Network area is an honest empty state" header | :1-2 | PARTIAL | area is STAGED (visible inert rows), not empty — but the test itself asserts no empty-state testid (:64), so the header is stale wording; the code under test agrees with the tests, not the header comment |
| "telemetry rows … relocated off the screen (g11-P2-1: ui=false)" | :4-5 | REAL | schema-truth test |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No component mocks; MockTransport only at seam; generated schema constants imported.

## 8. Open questions for the next agent
None blocking (F-001 is naming only).
