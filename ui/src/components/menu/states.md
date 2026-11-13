# Menu primitives — state & behavior tables (card 4.B3)

Components: `MenuBar`, `Dropdown`/`Select`, `ContextMenu`, `Tooltip`, plus the
internal shared `MenuPopup`/`MenuItem` rendering used by every menu surface.

**W26 figma skin** (frames `menu.svg`, `Profiles.svg`, `right-click menu.svg`,
`context menu.svg` + friends, `option.svg`, `Dropdown.svg`, `Setting
Preset.svg`; spec `design/figma-fidelity-spec.md` §2 “Menus”; current
grammar post f07/e12/i16 — w26.k02 doc-rot pass, j02 J02-6):
popups are 400px-class r10–12 panels (`--radius-menu`) on
`--color-surface-panel` with a 1px `--color-stroke-border` and the frame's
`--shadow-overlay` (the frames carry a drop shadow, so the “no shadows unless
framed” rule is satisfied). Rows are r6 (`--radius-control`), 48px comfortable
/ ~40px compact (w26.f07); hover AND selected rows fill
`--color-surface-selected` (#1f1f1f, w26.i16) with the label HELD quiet —
the fill is the state signal; labels are ≈15px (`--font-size-section`)
medium `--color-content-secondary` (#8F8F8F — the raw frames' value,
SVG-over-spec, w26.e12 F4); shortcuts are right-aligned ≈15px
`--color-content-accelerator` (#6b6b6b dark / #5f5f5f light — the quiet
band; w26.i19 token, contrast ruling in menu.css); the leading icon slot
(20px comfortable / 16px compact) and submenu chevron use
`--color-stroke-icon`, with menu icons on the Icon primitive's TWO-TONE face
(w26.i16; `iconAccent` is a row-level class hook, no longer a full-orange
override). Density prop (additive): `comfortable` (default) = big player
menus; `compact` = settings-class dropdowns. Entries may carry `icon`
(`IconName`) and a selective `iconAccent` flag (frames: profile/wand rows).

Data shape (doc 05 §8.3 — menus are DATA):
`{ id, labelKey, icon?, iconAccent?, accelerator?, submenu?, disabled?, checked? }`
— labels are i18n keys resolved through the `resolveLabel` prop (wire
`useMsg()` / `MessageKey` at the app layer); accelerators are display strings
produced from keymap data (doc 05 §8.1 binding-id indirection) and render
verbatim. The primitives never hardcode labels or keys.

## MenuBar

| State | Trigger | Menu item | Notes |
|---|---|---|---|
| rest | text `--color-content-secondary` | — | horizontal `role="menubar"`; trigger 32px (title-bar scale, W25 §2) |
| hover | `--color-surface-hover` fill | `--color-surface-selected` (r6 rows, w26.i16 #1f1f1f; label stays quiet) | |
| open | `aria-expanded="true"`, `--color-surface-selected` | — | popup `role="menu"`, r10–12 panel, `--color-surface-panel` |
| focus-visible | 2px accent outline | 2px accent outline | roving tabindex (doc 08 §4.5) |
| active (roving focus) | — | `--color-surface-selected` fill, label stays quiet (frames hold label tone) | keyboard nav moves the “active” row |
| disabled | n/a (menubar items have no disabled state) | `aria-disabled="true"`, `--opacity-disabled`, focusable but inert | WAI-ARIA allows focusable-disabled |
| checked | — | `menuitemcheckbox`/`menuitemradio` + `aria-checked`, accent ✓ glyph | |
| icon (W26, optional) | — | 20px two-tone line icon (16px compact); `iconAccent` = row hook for the two-tone face (w26.i16) | data-driven, additive |

Keyboard model (WAI-ARIA APG menubar/menu): ArrowLeft/Right move top items
(switching the open menu when one is open); ArrowDown/Enter/Space open with
FIRST-item focus; ArrowUp opens with LAST-item focus (w26.k02, j02 J02-9 —
APG menubar); ArrowUp/Down/Home/End navigate items; ArrowRight opens a
submenu (instant), ArrowLeft closes back to the parent; Escape closes and
**restores focus to the trigger**; Tab is trapped (wraps) inside an open
menu; separators are skipped by navigation; Enter/Space/click activate.

Hover intent for submenus: **open 150 ms**, close 100 ms
(`SUBMENU_OPEN_DELAY_MS` / `SUBMENU_CLOSE_DELAY_MS`) — [PROPOSAL]; doc 08
pins no value.

## Dropdown / Select

Trigger: `aria-haspopup="listbox"`, `aria-expanded`, accessible name from the
`label` prop; w26.k02 (j02 J02-2): the SELECTED VALUE is exposed to SRs via
`aria-describedby` → the visible value span (announced after the name) — the
name-restructured form (`"<label>: <value>"`) needs coordinated host-test
updates and is staged for the Lead (see the w26.k02 lane log); the open list
carries an id wired from the trigger via `aria-controls`. r6 recessed-well
surface with a 1px border + chevron (W26); 48px comfortable / 40px compact;
shows the selected option's resolved label. Open
list: `role="listbox"` + `role="option"` (`aria-selected`, `aria-disabled`),
r6 panel `--color-surface-panel` with 2px inset, roving tabindex. Options may
carry a leading icon (additive `icon`). Hover/focus rows r6
`--color-surface-hover`/`--color-surface-selected`; the selected option shows
an orange check (`--color-accent-icon`, frame `option.svg`). Arrows/Home/End
move (disabled options skipped); Enter/Space or click selects and closes with
focus restored to the trigger; Escape/Tab close + restore and are SCOPED to
the dropdown (stopPropagation — host dialogs listening for Escape survive,
the w26.g19 fix, regression-tested w26.k02); outside pointer
dismisses; **typeahead** accumulates printable chars for 500 ms
(`TYPEAHEAD_RESET_MS`, [PROPOSAL]) and jumps to the next label prefix match.
Invalid state: `--color-record` outline.

## ContextMenu

Right-click (`contextmenu`, default suppressed) opens the shared menu popup
at the pointer coordinates (fixed positioning, portal-free — doc 08 specifies
no portals). First item is focused on open; Escape closes and restores focus
to the previously-focused element; item activation dismisses; outside pointer
dismisses.

## Tooltip

`role="tooltip"` bubble linked via `aria-describedby` ON THE FOCUSABLE TRIGGER
(w26.k02, j02 J02-1 — the wiring rides the child through cloneElement, merging
any describedby the trigger already carries; the old wrapper-level attribute
was never announced). Hover intent: show **500 ms**, hide **150 ms**
(`TOOLTIP_SHOW_DELAY_MS` / `TOOLTIP_HIDE_DELAY_MS`, [PROPOSAL] — doc 08 pins
no value); keyboard focus shows immediately (APG guidance), blur hides;
Escape hides while focused and is CONSUMED while the tooltip is open
(stopPropagation, j02 J02-13 — a tooltiped control inside a dialog no longer
closes the dialog on Escape; closed-state Escape passes through).

## Known gaps / follow-ups

- **Raw-frame vs task-pin label color** [VERIFY — Lead call]: the raw SVG
  frames draw menu labels/shortcuts at `#8F8F8F`/`#6B6B6B`, while task w26.a05
  + spec §2 pin `#e8e8e8`/`#9e9e9e`. Implemented the task pins via the nearest
  semantic tokens (`--color-content-bright` / `--color-content-secondary`);
  if the Lead rules for the raw-frame values, swap to
  `--color-content-secondary` / `--color-content-tertiary`.
- **0.95 surface translucency** (frames: `#191919` @ 0.95 + backdrop blur 25)
  has no rgba token in the pipeline — popups render solid
  `--color-surface-panel` (flagged for the token pipeline; a `color-mix()`
  translucency could adopt it once a token exists).
- Context-menu coordinates ARE viewport-clamped since w26.f07 — every popup
  measures itself on mount (useLayoutEffect, pre-paint) and flips across the
  pointer / shifts to the 8px viewport margin (MenuPopup.tsx + the
  geometry-pinned tests in MenuPopup.test.tsx).
- Menubar menus have no typeahead (only Select does); APG marks it optional.
- `MenuItem` is presentational (click activation); keyboard handling lives in
  `MenuPopup` so all menu surfaces share one behavior implementation.
- Accelerators are part of each item's accessible name (screen readers
  announce shortcuts); tests therefore use prefix-regex role queries.
- No `forwardRef` HOC wrapper — React 19 ref-as-prop is used instead
  (preserves generics for the `labelKey` type parameter).
- W26: comfortable row height is 48px (w26.f07 re-pin to the raw frames'
  pitch at their 375px panel width; the earlier 58px figure came from the
  2.25×-scaled render measurements).
- W26: two-tone icons (gray outline + orange details) ARE expressible since
  w26.i16 — `MenuItem` renders the Icon primitive's `twoTone` face (#8F8F8F
  base + #AB6314 details); `iconAccent` remains the row-level data flag
  (class hook), no longer a full-orange override.
- W26: hover/selected menu rows hold the label color (frames change only the
  row fill); the dropdown selected option holds its label quiet too (w26.i16
  — g20-P3-2) — frames keep `#8F8F8F` there.
- Checkable rows render the check slot only — a leading `icon` is dropped on
  `checked`/radio rows (frame grammar; documented in types.ts, j02 J02-12).
