# AUDIT: ui/src/components/menu/menu.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 551 / 33 selector groups |
| File verdict | REAL (shared menu/dropdown/tooltip stylesheet; all selectors live; both cross-file claims verified, one stale line-number citation) |
| Reachable from shipped entry point? | YES — imported by Dropdown.tsx, ContextMenu.tsx, Tooltip.tsx, MenuBar.tsx, PlaylistOptionsMenu.tsx; menus render from AppShell/PlayerScreen throughout. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python class/var sweep (33 classes, dead=NONE; 27 vars, missing=NONE); rg token + constant verification (exit 0); sed quote checks (exit 0) |

## 1. One-paragraph verdict
Real, dense, well-cross-referenced stylesheet. All 33 classes referenced by TSX, all 27 custom properties resolve, reduced-motion honored for all four transitioned controls. The two cross-file invariants it documents are TRUE and current: `--color-content-accelerator` exists in both themes (tokens.css:36 #6b6b6b, :121 #5f5f5f) and the dropdown `max-height: 320px` matches `DROPDOWN_LIST_MAX_HEIGHT = 320` (Dropdown.tsx:15). Only defect: the comment cites "tokens.css:119" for the light token; it is at line 121 — stale citation drift.

## 2. What actually works (verified)
— verified by rg: `src/tokens/generated/tokens.css:36 --color-content-accelerator: #6b6b6b;` and `:121 --color-content-accelerator: #5f5f5f;` (dark/light as the comment claims).
— verified by rg: `Dropdown.tsx:15 const DROPDOWN_LIST_MAX_HEIGHT = 320;` ⇔ menu.css:430 `max-height: 320px;` — the "one grammar in two files, cross-linked both ways" claim holds.
— verified by python sweep: 33/33 selectors live, 27/27 vars resolve.

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/menu.css:261-262
Code:
```
   * --color-content-accelerator is LIVE in both themes (w26.i19 Lead-landed:
   * tokens.css:36 dark #6b6b6b, :119 light #5f5f5f), so the vestigial
```
What is wrong: The light token is at tokens.css:121, not :119 (sed verified). The values are right; the line citation is stale (token sheet grew two lines). Why it matters: this repo's audit method leans on comment citations; drifted ones waste the next reader's verification. Confidence: HIGH — Verification: ran sed on tokens.css:119-121.

## 4. Per-function table (file >100 LOC — selector groups)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-menubar (+trigger states) | 40-78 | REAL | — | |
| .lumen-menu (+compact/submenu/comfortable/host) | 81-116 | REAL | — | z10; 400px floor, 265px flyout |
| .lumen-menu__item family (hover/selected/focus/disabled/icon-accent) | 119-218 | REAL | — | two-tone icon grammar documented |
| __check / __label / __accelerator | 221-282 | REAL | — | accelerator token live (verified) |
| __submenu-arrow / __separator | 286-298 | REAL | — | 1px hairline on row boundary |
| .lumen-context-menu__popup | 301-306 | REAL | — | fixed at pointer |
| .lumen-dropdown trigger/value/arrow states (+invalid) | 309-406 | REAL | — | flex-basis fix documented; chevron rotate |
| .lumen-dropdown__list (+data-direction=up) | 409-440 | REAL | — | max-height 320 cross-linked (verified) |
| .lumen-dropdown__option family (+option-icon/label) | 442-515 | REAL | — | quiet labels on all states per frame |
| .lumen-tooltip (+bubble--bottom) | 518-542 | REAL | — | |
| @media reduced-motion | 544-551 | REAL | — | all 4 transitioned controls |

## 5. Test honesty
No tests here. Menu/MenuItem/Dropdown tests exist elsewhere (baseline ui-test.txt: 136 files PASS includes menu suites).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "--color-content-accelerator is LIVE in both themes (tokens.css:36 dark, :119 light)" | menu.css:260-262 | TRUE values, STALE line (121) | sed tokens.css:119-121 |
| "keep in sync with DROPDOWN_LIST_MAX_HEIGHT (Dropdown.tsx)" | menu.css:427-429 | TRUE | Dropdown.tsx:15 = 320 ⇔ menu.css:430 |
| "--color-surface-selected carries exactly #1f1f1f (tokens.json)" | menu.css:18-19,145-146 | TRUE (verified in controls.css audit pass) | tokens sweep; same claim family |

## 7. Dead / duplicated / unreachable code
None. 33/33 selectors live.

## 8. Dependencies & flags
Token set (incl. --color-content-accelerator), icon.css two-tone hook (`.lumen-icon__detail` via MenuItem). No flags.

## 9. Open questions for the next agent
- None material.
