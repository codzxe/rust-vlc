# AUDIT: ui/src/screens/player/dialogs/dialogs.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 544 / ~11 selector groups (player-dialog surface/header/body/rows/list/footer, tools-dialog cards/tabs/staged, tools-card, tools-row, motion guards, light-mode holds) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by GotoTimeDialog.tsx:43 and ToolsDialogShell.tsx:43; GotoTimeDialog mounted from ControlBar.tsx:404, ToolsDialogHost mounted by PlayerScreen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -l` producers for `player-dialog__row-main`/`tools-dialog__staged-tool`/`tools-card__toggle`/`tools-row__value` (ToolsDialog/ToolsDialogShell/BookmarksDialog/GotoTimeDialog); `rg -n "dialogs.css"` (2 imports + harness.test.tsx reads it); full read |

## 1. One-paragraph verdict
Real stylesheet, every selector group produced by the dialogs components (GotoTime/Bookmarks/ToolsDialog shell + cards), with per-value frame citations and a focus-order trick (header `order: -1` after DOM-later body so first focus lands on a body control, L69-77) that the shell documents and tests read. The one hygiene item: the `.player-dialog__error` inline error line exists because the dialogs bypass the provider's dispatch (comment L201-206) — a documented architectural seam, not a CSS defect, but it means dialog command failures surface only inside the dialog (see ToolsDialog audit).

## 2. What actually works (verified)
— verified by `rg -l`: all four spot-checked grammar families produced by 4 real TSX files; harness.test.tsx includes dialogs.css in a file list it verifies (line 29). Baseline vitest 1705 passed / npm build pass (baseline §13).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/dialogs/dialogs.css:201-211
Code:
```
/* Inline command-failure line (w26.k09, j09 P2): the dialog's own sends
 * (load / add / delete) bypass the provider's dispatch, so the player's
 * lastError surface never sees them — this line IS the error surface.
```
What is wrong: architecture note, not a styling bug — dialog-originated command failures are invisible outside the dialog lifetime (close the dialog, lose the error). Disclosed honestly; blast radius limited to dialog sends. Confidence: HIGH — Verification: read.

## 4. Per-function table (selector groups)
| Group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-dialog.player-dialog__surface | 38-57 | REAL | — | 440px compact / 400px tools (`:has()` size pin) |
| .player-dialog header/title/close | 59-120 | REAL | — | order:-1 focus trick; light-mode re-recess |
| body/subtitle/form/hint/empty/error | 122-211 | REAL | S4 | F-001 error-surface seam |
| bookmarks list rows | 216-286 | REAL | — | hover band, full-row quiet jump button |
| footer | 294-311 | REAL | — | 100×32 frame actions, only-child right pin |
| tools-dialog cards/tabs/panels | 315-356 | REAL | — | `:has(.eq-tool--open)` staged-note reveal |
| tools-card toggle/body | 388-461 | REAL | — | frame 360×48; elevated fill rationale documented |
| tools-row + value chip | 465-528 | REAL | — | light-mode wells held |
| motion guards | 538-544 | REAL | — | reduced-motion covers animated nodes |

## 5. Test honesty
harness.test.tsx lists this CSS among verified files (rg line 29). Mutation reasoning: removing the `order: -1` rule would break the documented focus-order test in the shell's test suite; not individually executed — covered by baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the surface trap's first-focusable lands on a body control, never the ✕" (L72-75) | comment | REAL-UNPROVEN | depends on ToolsDialogShell DOM order + focus trap; test coverage asserted in shell comments; not executed here |
| Frame pins (440/400/50px/100×32) | comments | REAL | values present at L39/56/83/305 |

## 7. Dead / duplicated / unreachable code
None — all grammars produced.

## 8. Dependencies & flags
Semantic tokens; `:has()` used twice (L55, L378) — same webview-engine caveat as form.css. No feature flags.

## 9. Open questions for the next agent
- None for CSS. The dialog-bypasses-provider dispatch seam (F-001) belongs to the ToolsDialog/BookmarksDialog audits.
