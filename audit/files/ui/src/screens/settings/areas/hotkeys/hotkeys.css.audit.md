# AUDIT: ui/src/screens/settings/areas/hotkeys/hotkeys.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 173 / 13 selector groups |
| File verdict | REAL (dialog chrome + 141-binding table + dirty/conflict state styles; all classes live, all vars resolve) |
| Reachable from shipped entry point? | YES — imported by HotkeysEditor.tsx, mounted inside the controls-area dialog (areas/controls.tsx) → SettingsScreen → /settings routes → App. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | class sweep over hotkeys dir (13/13 hotkeys-* + visually-hidden classes referenced in HotkeysEditor.tsx); var sweep (18/18 resolve); baseline ui-test.txt:118,171,173 — HotkeysEditor 8 + hotkeysData 2 + conflicts 7 tests PASS |

## 1. One-paragraph verdict
Clean frame-cited stylesheet: 1048×800 dialog deck, 50px topbar, single scroller (the documented fix for a 5780px inline slab), dirty-gated apply bar neutralizing the settings-applybar sticky rule inside the dialog, changed/conflict row states pinned via data attributes, and a scoped `.visually-hidden` (previously a global collision risk, fix cited). Zero findings — everything checked and everything resolved.

## 2. What actually works (verified)
— verified by sweep: every selector (hotkeys-dialog__*, hotkeys-editor__*, hotkeys-table/-row__*, .visually-hidden scoped) referenced in the hotkeys TSX; 18/18 custom properties defined in shipped CSS.
— verified by baseline: HotkeysEditor (8), hotkeysData (2), conflicts (7) tests PASS — the dirty/conflict data attributes this sheet styles are real editor states.

## 3. Findings
None. Checked: dead selectors (none), unresolved vars (none), raw colors (none — all tokens), specificity order (two conscious stylelint suppressions with justifications at the conflict/base rule crossings), reduced-motion (the only transitions are 120ms color fades on the close button — no guard, cosmetic only).

## 4. Per-function table (file >100 LOC — selector groups)
| Selector group | Lines | Verdict | Notes |
|---|---|---|---|
| .lumen-dialog.hotkeys-dialog deck | 11-20 | REAL | min() clamps |
| __topbar/__title/__close (+states) | 22-62 | REAL | focus-visible ring |
| .hotkeys-editor__body scroller | 64-69 | REAL | the one scroll container |
| __applybar neutralization | 71-83 | REAL | static footer, border-top only |
| .hotkeys-table th/td | 85-96 | REAL | hairline row separators |
| .hotkeys-row[data-changed]/[data-conflict] | 98-106 | REAL | accent borders |
| __input (+disabled) | 107-127 | REAL | suppressions justified |
| __conflict/__command/__mode/__reset | 129-148 | REAL | caption floor 11px, AA note |
| .hotkeys-table .visually-hidden | 164-173 | REAL | scoped fix (k13 P3-10), clip-path modern |

## 5. Test honesty
Covered by the three hotkeys suites (baseline PASS).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 141-binding table never grows the page" | hotkeys.css:64-65 | TRUE (CSS side) | single overflow-y:auto body; binding count is hotkeysData's concern (2 tests PASS) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Dialog primitive chrome, settings-applybar grammar, token set. No flags.

## 9. Open questions for the next agent
- None.
