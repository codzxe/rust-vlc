# Menu primitives — state & behavior tables (card 4.B3)

Components: `MenuBar`, `Dropdown`/`Select`, `ContextMenu`, `Tooltip`, plus the
internal shared `MenuPopup`/`MenuItem` rendering used by every menu surface.

Data shape (doc 05 §8.3 — menus are DATA):
`{ id, labelKey, accelerator?, submenu?, disabled?, checked? }` — labels are
i18n keys resolved through the `resolveLabel` prop (wire `useMsg()` /
`MessageKey` at the app layer); accelerators are display strings produced from
keymap data (doc 05 §8.1 binding-id indirection) and render verbatim. The
primitives never hardcode labels or keys.

## MenuBar

| State | Trigger | Menu item | Notes |
|---|---|---|---|
| rest | text `--color-content-secondary` | — | horizontal `role="menubar"` |
| hover | `--color-surface-hover` fill | `--color-surface-hover` (48px rows, rx6) | |
| open | `aria-expanded="true"`, `--color-surface-selected` | — | popup `role="menu"`, `--radius-menu`, `--color-surface` |
| focus-visible | 2px accent outline | 2px accent outline | roving tabindex (doc 08 §4.5) |
| disabled | n/a (menubar items have no disabled state) | `aria-disabled="true"`, `--opacity-disabled`, focusable but inert | WAI-ARIA allows focusable-disabled |
| checked | — | `menuitemcheckbox`/`menuitemradio` + `aria-checked`, accent ✓ glyph | |

Keyboard model (WAI-ARIA APG menubar/menu): ArrowLeft/Right move top items
(switching the open menu when one is open); ArrowDown/Enter/Space open;
ArrowUp/Down/Home/End navigate items; ArrowRight opens a submenu (instant),
ArrowLeft closes back to the parent; Escape closes and **restores focus to
the trigger**; Tab is trapped (wraps) inside an open menu; separators are
skipped by navigation; Enter/Space/click activate.

Hover intent for submenus: **open 150 ms**, close 100 ms
(`SUBMENU_OPEN_DELAY_MS` / `SUBMENU_CLOSE_DELAY_MS`) — [PROPOSAL]; doc 08
pins no value.

## Dropdown / Select

Trigger: `aria-haspopup="listbox"`, `aria-expanded`, accessible name from the
`label` prop; shows the selected option's resolved label. Open list:
`role="listbox"` + `role="option"` (`aria-selected`, `aria-disabled`), roving
tabindex. Arrows/Home/End move (disabled options skipped); Enter/Space or
click selects and closes with focus restored to the trigger; Escape/Tab close
+ restore; outside pointer dismisses; **typeahead** accumulates printable
chars for 500 ms (`TYPEAHEAD_RESET_MS`, [PROPOSAL]) and jumps to the next
label prefix match. Invalid state: `--color-record` outline.

## ContextMenu

Right-click (`contextmenu`, default suppressed) opens the shared menu popup
at the pointer coordinates (fixed positioning, portal-free — doc 08 specifies
no portals). First item is focused on open; Escape closes and restores focus
to the previously-focused element; item activation dismisses; outside pointer
dismisses.

## Tooltip

`role="tooltip"` bubble linked via `aria-describedby` on the wrapper. Hover
intent: show **500 ms**, hide **150 ms** (`TOOLTIP_SHOW_DELAY_MS` /
`TOOLTIP_HIDE_DELAY_MS`, [PROPOSAL] — doc 08 pins no value); keyboard focus
shows immediately (APG guidance), blur hides; Escape hides while focused.

## Known gaps / follow-ups

- **0.95 surface translucency** (doc 08 §115 context menu) has no rgba token
  in the 4.A2 pipeline — popups render solid `--color-surface` (flagged for
  the token pipeline).
- Context-menu coordinates are not viewport-clamped (jsdom has no layout;
  needs a measured clamp when composed into real screens).
- Menubar menus have no typeahead (only Select does); APG marks it optional.
- `MenuItem` is presentational (click activation); keyboard handling lives in
  `MenuPopup` so all menu surfaces share one behavior implementation.
- Accelerators are part of each item's accessible name (screen readers
  announce shortcuts); tests therefore use prefix-regex role queries.
- No `forwardRef` HOC wrapper — React 19 ref-as-prop is used instead
  (preserves generics for the `labelKey` type parameter).
