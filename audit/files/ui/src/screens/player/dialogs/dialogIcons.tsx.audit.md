# AUDIT: ui/src/screens/player/dialogs/dialogIcons.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 73 / 2 components |
| File verdict | REAL (two frame-mined stroke glyphs, currentColor-tokened, consumed by all three player dialogs) |
| Reachable from shipped entry point? | YES — BookmarksDialog.tsx, ToolsDialogShell.tsx, GotoTimeDialog.tsx (player dialogs → PlayerScreen). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers (3 dialog files); read |

## 1. One-paragraph verdict
Two tiny presentational SVGs (close ✕, chevron) with frame-cited geometry, currentColor inking (token-driven), aria-hidden/focusable=false, and a documented rationale for living outside the shared icon set (Lead follow-up recorded). Zero findings — compact audit.

## 2. What actually works (verified)
— rg: all three player dialogs consume the glyphs.

## 3. Findings
None. Checked: consumers, a11y attributes, stroke/units arithmetic comments (15/24 at size 16 = 10px — consistent), no dead exports.

## 4. Per-function table
n/a (<100 LOC).

## 5. Test honesty
Dialog suites (baseline PASS) render these glyphs.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "glyphs the … shared icon set … does not carry yet" | dialogIcons.tsx:2-4 | TRUE (as of tree) | local module; shared set addition is a recorded follow-up |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None. No flags.

## 9. Open questions for the next agent
- None.
