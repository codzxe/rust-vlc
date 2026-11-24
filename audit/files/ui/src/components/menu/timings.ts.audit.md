# AUDIT: ui/src/components/menu/timings.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 26 / 0 (5 exported timing constants) |
| File verdict | REAL (constants consumed by the menu family + their tests) |
| Reachable from shipped entry point? | YES — imported by Dropdown.tsx / MenuPopup.tsx (menu components used app-wide) and MenuBar/Tooltip tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; grep consumers |

## 1. One-paragraph verdict
Five hover-intent/typeahead timing constants, isolated in their own module for react-refresh compliance (documented, issue #36), consumed by the real menu components. All five carry `[PROPOSAL — states.md]` markers — the values are recorded engineering choices, not design-doc-pinned. Clean.

## 2. What actually works (verified)
— verified by grep: Dropdown.tsx, MenuPopup.tsx + two test files import the exact constants.

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/timings.ts:11-24
Code:
```
/** Typeahead buffer lifetime before the search resets [PROPOSAL — states.md]. */
export const TYPEAHEAD_RESET_MS = 500;
```
What is wrong: every value is a [PROPOSAL] — no design doc pins them; they are honest, disclosed defaults. Hygiene-grade only. Confidence: HIGH — Verification: read.

## 4. Per-function table
n.a. — constants module.

## 5. Test honesty
Tests import the constants directly (advance-timers-by exact values) — timing behavior is test-pinned.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Doc 08 pins none of these values; states.md is their source of truth" | L9-10 | REAL (disclosure) | consistent with the [PROPOSAL] markers |

## 7. Dead / duplicated / unreachable code
None — all five constants consumed.

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
None.
