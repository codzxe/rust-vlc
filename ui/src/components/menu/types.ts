/**
 * Card 4.B3 — menu primitives data shapes (doc 05 §8.3: menus are DATA).
 *
 * Every leaf is `{ id, labelKey, accelerator?, submenu?, disabled?, checked? }`.
 * - `labelKey` is an i18n message key, resolved through the `resolveLabel`
 *   seam (wire `useMsg()` / `MessageKey` at the app layer). The primitive
 *   never hardcodes labels.
 * - `accelerator` is a display string produced from keymap data (doc 05 §8.1
 *   binding-id indirection — a user rebinding updates menus automatically).
 *   The primitive renders it as-is; it does not interpret keys.
 * - Dynamic submenus (tracks, devices, chapters) arrive as data at runtime —
 *   same shape, populated by the app layer from queries.
 */

export interface MenuEntryBase<K extends string = string> {
  /** Stable entry id (DOM wiring + tests). */
  id: string;
  /** i18n message key — resolved via `resolveLabel` (doc 05 §8.3: keys only). */
  labelKey: K;
  /** Accelerator display string from keymap data; rendered verbatim. */
  accelerator?: string;
  /** Nested menu; presence implies a submenu affordance. */
  submenu?: MenuEntry<K>[];
  /** Disabled from data (e.g. command outside the feature set — doc 05 §6.4). */
  disabled?: boolean;
  /** Checked state for checkable items (renders `menuitemcheckbox`). */
  checked?: boolean;
  /** Explicit item kind; `checked` without a kind implies `"checkbox"`. */
  kind?: "item" | "checkbox" | "radio";
  /** Fires on activation (click / Enter / Space) when enabled. */
  onSelect?: () => void;
}

export interface MenuSeparator {
  id: string;
  kind: "separator";
}

export type MenuEntry<K extends string = string> = MenuSeparator | MenuEntryBase<K>;

/** Label resolver seam — the app layer passes `useMsg()`. */
export type ResolveLabel<K extends string = string> = (key: K) => string;

/** Default resolver: identity (keys rendered verbatim — dev/sandbox mode). */
export const identityResolveLabel = <K extends string>(key: K): string => key;
