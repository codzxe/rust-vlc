# Menu primitives — state & behavior tables (card 4.B3)

Components: `MenuBar`, `Dropdown`/`Select`, `ContextMenu`, `Tooltip`, plus the
internal shared `MenuPopup`/`MenuItem` rendering used by every menu surface.

**W26 figma skin** (frames `menu.svg`, `Profiles.svg`, `right-click menu.svg`,
`context menu.svg` + friends, `option.svg`, `Dropdown.svg`, `Setting
Preset.svg`; spec `design/figma-fidelity-spec.md` §2 “Menus”; task w26.a05):
popups are 400px-class r10–12 panels (`--radius-menu`) on
`--color-surface-panel` with a 1px `--color-stroke-border` and the frame's
`--shadow-overlay` (the frames carry a drop shadow, so the “no shadows unless
framed” rule is satisfied). Rows are r6 (`--radius-control`) with hover
`--color-surface-hover` and the roving active row `--color-surface-selected`;
labels are `--font-size-label` medium in `--color-content-bright` (task pin
#e8e8e8 — nearest semantic token #eaeaea); shortcuts are right-aligned
`--font-size-body` `--color-content-secondary` (task pin #9e9e9e — nearest
semantic token #8F8F8F); the leading icon slot and submenu chevron use
`--color-stroke-icon` (frames stroke #8F8F8F). Density prop (additive):
`comfortable` (default) = big player menus (58px rows, 22px icon slot, 16px
medium labels, 14px shortcuts); `compact` = settings-class dropdowns (~40px
rows, 16px icons, 14px labels). Entries may carry `icon` (`IconName`) and a
selective `iconAccent` flag (orange `--color-accent-icon` — profile/wand
frames).

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
| hover | `--color-surface-hover` fill | `--color-surface-hover` (r6 rows) | |
| open | `aria-expanded="true"`, `--color-surface-selected` | — | popup `role="menu"`, r10–12 panel, `--color-surface-panel` |
| focus-visible | 2px accent outline | 2px accent outline | roving tabindex (doc 08 §4.5) |
| active (roving focus) | — | `--color-surface-selected` fill + bright label | keyboard nav moves the “active” row |
| disabled | n/a (menubar items have no disabled state) | `aria-disabled="true"`, `--opacity-disabled`, focusable but inert | WAI-ARIA allows focusable-disabled |
| checked | — | `menuitemcheckbox`/`menuitemradio` + `aria-checked`, accent ✓ glyph | |
| icon (W26, optional) | — | 22px `--color-stroke-icon` line icon; `iconAccent` → `--color-accent-icon` | data-driven, additive |

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
`label` prop; r6 recessed-well surface with a 1px border + chevron (W26); 48px
comfortable / 40px compact; shows the selected option's resolved label. Open
list: `role="listbox"` + `role="option"` (`aria-selected`, `aria-disabled`),
r6 panel `--color-surface-panel` with 2px inset, roving tabindex. Options may
carry a leading icon (additive `icon`). Hover/focus rows r6
`--color-surface-hover`/`--color-surface-selected`; the selected option shows
an orange check (`--color-accent-icon`, frame `option.svg`). Arrows/Home/End
move (disabled options skipped); Enter/Space or click selects and closes with
focus restored to the trigger; Escape/Tab close + restore; outside pointer
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

`role="tooltip"` bubble linked via `aria-describedby` on the wrapper. Hover
intent: show **500 ms**, hide **150 ms** (`TOOLTIP_SHOW_DELAY_MS` /
`TOOLTIP_HIDE_DELAY_MS`, [PROPOSAL] — doc 08 pins no value); keyboard focus
shows immediately (APG guidance), blur hides; Escape hides while focused.

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
- Context-menu coordinates are not viewport-clamped (jsdom has no layout;
  needs a measured clamp when composed into real screens).
- Menubar menus have no typeahead (only Select does); APG marks it optional.
- `MenuItem` is presentational (click activation); keyboard handling lives in
  `MenuPopup` so all menu surfaces share one behavior implementation.
- Accelerators are part of each item's accessible name (screen readers
  announce shortcuts); tests therefore use prefix-regex role queries.
- No `forwardRef` HOC wrapper — React 19 ref-as-prop is used instead
  (preserves generics for the `labelKey` type parameter).
- W26: comfortable row height is 58px (task pin; the raw frames measure ~48px
  at their 375px panel width — the task/W25-spec 58–60px figure comes from the
  2.25×-scaled render measurements). App layers wanting frame-exact 48px
  pitch can override via CSS until a density variant is requested.
- W26: two-tone icons (gray outline + orange details, as in the frames) are
  not expressible with the single-color `Icon` primitive; `iconAccent`
  approximates with a full-accent icon.
- W26: hover/selected menu rows hold the label color (frames change only the
  row fill); the dropdown selected option brightens its label for affordance
  — frames keep `#8F8F8F` there (minor, deliberate).
