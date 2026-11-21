# AUDIT: ui/src/components/menu/MenuItem.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 87 / 1 (MenuItem) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (sole production consumer: MenuPopup.tsx:329 renders it for every menu row — MenuBar menus, ControlBar's app menu (open-recent, profile flyout), context menus; all mounted from PlayerScreen/AppShell routes) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read full file + types.ts (the MenuEntry contract) + MenuItem.test.tsx (66 lines) · `rg -n "MenuItem" ui/src` (consumer closure: MenuPopup + tests + barrel) · `rg -n "disabled\|onSelect\|Enter\|Space" MenuPopup.tsx` (:212 activation returns early on disabled — the "when enabled" contract is upheld by the consumer; :275 Enter/Space routed through it) · menu.css check-selector verification from the ProfilesFlyout audit (check paints at :165-166) |

## 1. One-paragraph verdict
A tight, fully-presentational row renderer that keeps every behavioral promise it inherits: roles derive correctly from data (menuitem / menuitemcheckbox / menuitemradio), the roving tabindex, aria-selected/current/haspopup/expanded wiring is right, disabled rows drop their click handler while the owning popup independently refuses keyboard activation for them (verified MenuPopup.tsx:212), and the slot grammar (icon vs check slot vs both-absent) implements exactly the documented w26.k02 J02-12 rule — icon dropped on checkable rows, check slot for icon-less rows, both pinned by tests. The single finding is a small accessible-name defect: the accelerator span is rendered as plain text inside the menuitem, so "Ctrl+O" lands in the computed accessible name (the test suite's own regex accommodation proves it), which screen readers will announce as part of every row name.

## 2. What actually works (verified)
— Role derivation: `kind === "radio"` → menuitemradio; checkable (checkbox kind or `checked` present) → menuitemcheckbox with aria-checked; else menuitem — matches the documented shape ("checked without a kind implies checkbox", types.ts:62-63).
— The J02-12 slot rule is implemented and documented at both ends (types.ts:18-24 note ↔ :74-79 render conditions) and test-pinned: icon rows render the decorative icon (aria-hidden, :18-26), checkable rows keep the check slot and drop the icon (:41-50), plain rows keep the empty check slot for label alignment (also pinned from the consumer side in ProfilesFlyout.test.tsx:153-157).
— Disabled contract: `onClick={entry.disabled ? undefined : onClick}` + `aria-disabled` here, plus the consumer's activation guard (MenuPopup.tsx:212 `entry.disabled` early-return before `entry.onSelect?.()`) — focusable-but-inert, exactly as MenuPopup's header documents.
— The w26.i16 selected face: `entry.selected` → `--selected` class + `aria-current="true"`, deliberately NOT aria-checked (current-state marker) — pinned by ProfilesFlyout.test.tsx:163-170.
— Accelerator + submenu chevron affordances (:81-84) pinned by MenuItem.test.tsx:52-65.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/menu/MenuItem.tsx:81-83
Code:
```
      {entry.accelerator !== undefined && (
        <span className="lumen-menu__accelerator">{entry.accelerator}</span>
      )}
```
What is wrong: the accelerator span is not aria-hidden, so its text joins the computed accessible name of the menuitem ("Open File… Ctrl+O"). The component's own test proves the pollution: MenuItem.test.tsx:60 must use `name: /^Open File/` (regex) rather than the exact "Open File…" — the full accessible name contains "Ctrl+O".
Why it matters: screen readers announce the shortcut as part of every row name (redundant for sighted keyboard users, noise for SR users on every row of every menu); the ARIA menu pattern's examples typically exclude display-only shortcut text from the name.
What correct looks like: `aria-hidden="true"` on the span (it is display chrome, like the icon slot and the submenu arrow, both of which ARE aria-hidden).
Confidence: MEDIUM — Verification: read + the test's own regex accommodation; SR announcement behavior not executable here. The counter-argument (announcing shortcuts helps SR users learn them) keeps this at S4.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MenuItem | 42-87 | REAL | S4 | presentational only (keyboard owned by MenuPopup, verified); role/aria derivation correct; J02-12 slot rule correct; F-001 accelerator in accessible name |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| MenuItem.test.tsx:18 icon in decorative slot, label is the name | YES | icon presence + aria-hidden asserted |
| :28 icon-accent class flag | YES | class assertion |
| :41 check slot for checkable + icon-less rows; icon absent | YES | slot presence/absence assertions both ways |
| :52 accelerator text + chevron + aria-haspopup/expanded | YES | content + attribute assertions (regex name = F-001 evidence) |
Consumer-side pins: ProfilesFlyout.test.tsx:131-170 (selected face, aria-current vs aria-checked, +New icon row). All green: baseline ui-vitest.log.txt (1705 passed).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "checkbox / `checked` present → `menuitemcheckbox` + `aria-checked`; radio → `menuitemradio`" | header :23-27 | REAL | :52-61; tests :41-50 |
| "a CHECKABLE entry that ALSO carries an `icon` renders the check slot ONLY — the icon is dropped" | types.ts:18-24 | REAL | :74-79; test :41-50 |
| "keyboard navigation … is owned by the containing popup (MenuPopup)" | header :7-9 | REAL | MenuPopup.tsx:212, :275 |
| "`entry.selected` paints the resting selected face … carries aria-current (…not a checkable one)" | :19-21 | REAL | :62, :68; ProfilesFlyout.test.tsx:163-170 |
| "Roles per WAI-ARIA" | :23 | REAL with the F-001 caveat | name pollution from the accelerator span |

## 7. Dead / duplicated / unreachable code
- None. Every prop is consumed (onClick, itemRef, id, hasSubmenu, submenuOpen wired by MenuPopup).

## 8. Dependencies & flags
- Icon primitive (twoTone), MenuEntry types. No engine dependency (pure presentation), no feature flags.

## 9. Open questions for the next agent
- MenuPopup.tsx owns roving focus, the Tab trap, and submenu recursion — its auditor should confirm the hoverEnter path (:281) syncs the roving `active` index that this component's tabIndex consumes.
