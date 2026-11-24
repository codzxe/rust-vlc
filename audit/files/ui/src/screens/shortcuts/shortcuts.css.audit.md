# AUDIT: ui/src/screens/shortcuts/shortcuts.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 476 / 0 (stylesheet, 26 selectors) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported at ui/src/screens/shortcuts/ShortcutsScreen.tsx:62 (`import "./shortcuts.css";`) -> ShortcutsScreen routed from App -> main.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "shortcuts.css|shortcuts__" ui/src/screens/shortcuts/*.tsx`; python3 selector-vs-TSX class cross-check; `sed -n '396,404p;500,510p' ShortcutsScreen.tsx` |

## 1. One-paragraph verdict
A real, frame-measured stylesheet for the keyboard-shortcuts reference dialog.
All 26 selectors (`.shortcuts`, `.shortcuts__topbar` … `.shortcuts__save`, plus
`.lumen-dialog.shortcuts-dialog` overrides) are consumed by
ShortcutsScreen.tsx:402-505; no orphan selectors, no dead rules. It is honest
about what it styles: a permanently disabled Save button and a decorative
(pointer-events: none) edit pencil in the empty Global Hotkey column. The
hostile finding lives in the TSX, not here: the CSS merely styles a dead button.

## 2. What actually works (verified)
— python3 cross-check: 26/26 CSS class selectors appear as classNames in
ShortcutsScreen.tsx (rg output L402-505). — `import "./shortcuts.css"` present
(ShortcutsScreen.tsx:62). — Responsive fallback @media (width <= 1120px)
(css:466-476) matches the Dialog 92vw cap described at css:31-36.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/shortcuts/shortcuts.css:429-434 (and ShortcutsScreen.tsx:504)
Code:
```
.shortcuts__save.lumen-button:disabled {
  color: var(--color-accent);
  background: var(--color-accent-tinted);
  border-color: transparent;
  opacity: var(--opacity-disabled); /* frame: group at 50% */
}
```
What is wrong: the sheet ships a dedicated, carefully tuned style for a Save
button that is hardcoded `disabled` at ShortcutsScreen.tsx:504
(`<Button variant="tinted" className="shortcuts__save" disabled>`). There is no
state in which this button is enabled — the CSS comments admit it ("disabled —
read-only reference", css:21).
Why it matters: polish invested in a dead control; the dialog pretends to offer
saving it cannot do (ties into Wave 11's 143/150 dead hotkeys — this reference
screen cannot edit any binding).
What correct looks like: omit the footer Save entirely, or wire it to the
hotkeys settings area.
Confidence: HIGH — Verification: read + sed of both files.

## 4. Per-function table
N/A (stylesheet). Selector inventory: 26 class selectors + 2
`:root[data-theme="light"]` sweeps (css:183, 441, 455) + 1 media query; all
verified consumed (section 2).

## 5. Test honesty
ShortcutsScreen.test.tsx queries `.shortcuts__row/.shortcuts__action/.shortcuts__keycap/.shortcuts__combo/a.shortcuts__edit`
(test L42-134) — the tested DOM really carries these classes, so the CSS-covered
markup is under test (behavioral testing of CSS itself is out of vitest scope).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Save 100×32 ... (disabled — read-only reference)" | css:20-21 | REAL | TSX:504 `disabled` hardcoded |
| "the Hotkey column's pencil links to Settings → Hotkeys" | css:361-363 | REAL-UNPROVEN | TSX renders `a.shortcuts__edit`; navigation target not verified here (ShortcutsScreen audit owns it) |

## 7. Dead / duplicated / unreachable code
None in this file. (The disabled-Save styling is consumed, it just styles a dead
control — F-001.)

## 8. Dependencies & flags
Token-only styling (var(--color-*), --radius-*, --motion-*); no imports; no
feature gates. `color-mix()` requires a modern browser — consistent with the rest
of the UI sheets.

## 9. Open questions for the next agent
- Does the `a.shortcuts__edit` link actually navigate to the settings hotkeys
  area, or is it another dead surface? (ShortcutsScreen.tsx audit.)
