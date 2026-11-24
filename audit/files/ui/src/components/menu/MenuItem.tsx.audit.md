# AUDIT: ui/src/components/menu/MenuItem.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 87 / 1 component |
| File verdict | REAL (presentational ARIA menu item; roles/state wiring correct) |
| Reachable from shipped entry point? | YES — rendered by MenuPopup.tsx:329 (audited this wave) and ProfilesFlyout.tsx; menus throughout AppShell/Player. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; consumer rg (MenuPopup.tsx:3-4 import + :329 render; ProfilesFlyout); baseline ui-test.txt:154 MenuItem 4 tests PASS |

## 1. One-paragraph verdict
Correct presentational item: role mapping (menuitem / menuitemcheckbox / menuitemradio per entry kind), roving tabindex driven by the container's `active`, aria-checked only for checkable kinds, aria-current for the selected face, aria-haspopup/expanded for submenu parents, disabled gating via onClick-undefined + aria-disabled (focusable-but-inert, matching MenuPopup's keyboard behavior). Zero findings — compact audit per <100 LOC rule.

## 2. What actually works (verified)
— read + cross-check with MenuPopup (this wave): itemRef/active/hasSubmenu/submenuOpen are the exact props the popup passes; baseline MenuItem 4 tests PASS.

## 3. Findings
None. Checked: role correctness, conditional aria states (no aria-checked on plain items — `undefined`), two-tone icon flag, disabled click gating, class composition.

## 4. Per-function table
n/a (<100 LOC).

## 5. Test honesty
MenuItem.test.tsx (4, baseline PASS) pins role/aria rendering.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keyboard navigation … is owned by the containing popup" | MenuItem.tsx:7-9 | TRUE | no key handlers here; MenuPopup owns them (audited) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Icon (twoTone), types. No flags.

## 9. Open questions for the next agent
- None.
