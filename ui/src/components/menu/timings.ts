// Menu-family timing constants — the [PROPOSAL] defaults recorded in
// states.md. They live in their own module (not next to the components) so
// the component files only export components (react-refresh/
// only-export-components, issue #36) while tests keep importing the exact
// delays they advance timers by.
//
// Doc 08 pins none of these values; states.md is their source of truth:
//   * typeahead buffer reset (Dropdown)
//   * submenu hover-intent open/close (MenuPopup)
//   * tooltip show/hide hover-intent (Tooltip — keyboard focus shows
//     immediately, WAI-ARIA APG: never delay for keyboard users)

/** Typeahead buffer lifetime before the search resets [PROPOSAL — states.md]. */
export const TYPEAHEAD_RESET_MS = 500;

/** Submenu open on hover after a short delay, never instantly [PROPOSAL — states.md]. */
export const SUBMENU_OPEN_DELAY_MS = 150;

/** Submenu close delay, tolerant of sloppy pointer paths [PROPOSAL — states.md]. */
export const SUBMENU_CLOSE_DELAY_MS = 100;

/** Tooltip show hover-intent delay [PROPOSAL — states.md]. */
export const TOOLTIP_SHOW_DELAY_MS = 500;

/** Tooltip hide hover-intent delay [PROPOSAL — states.md]. */
export const TOOLTIP_HIDE_DELAY_MS = 150;
