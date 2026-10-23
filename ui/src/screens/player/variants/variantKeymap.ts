// Card 4.F2 — variant-switching keymap seam (doc 05 §8.1 / doc 07 §7.3).
//
// [PROPOSAL]: keymaps/default.toml defines NO variant-toggle binding today
// (verified — no miniplayer/music/layout entry exists in that file), and the
// v1 command catalog has no layout-switch command (it is UI-owned state per
// doc 08 §4 "UI-owned: layout mode (music/hide-UI/mini)"). This module seeds
// the two shell-owned recipe bindings in the same TOML format; when the
// keymap loader lands, these rows move into keymaps/default.toml verbatim
// and this seed is deleted. Keys avoid every key claimed in
// keymaps/default.toml's player-focused mode (Space/S/N/P/M/E/F/I/Esc and
// the media keys); final keys belong to the keymap data file, not code.

import { DEFAULT_KEYMAP, parseKeymap } from "../keymap";
import type { KeyBinding } from "../keymap";

/** Seed rows for the two variant-toggle recipes (see header [PROPOSAL]). */
export const VARIANT_KEYMAP_TOML = `# [PROPOSAL] variant-toggle bindings (card 4.F2 shell seam) — to be merged
# into keymaps/default.toml by the keymap loader card.
[player-variants]
miniplayer-toggle = { key = "B", recipe = "miniplayer-toggle", mode = "player-focused" }
music-mode-toggle = { key = "G", recipe = "music-mode-toggle", mode = "player-focused" }
`;

/** The parsed variant seed (module-load; a bad seed is a programmer error). */
export const VARIANT_KEYMAP: ReadonlyMap<string, KeyBinding> = parseKeymap(VARIANT_KEYMAP_TOML);

/**
 * Merge binding maps (base first, overlay wins). Duplicate ids across the
 * two maps are a programmer error — loud, never silently shadowed.
 */
export function mergeKeymaps(
  base: ReadonlyMap<string, KeyBinding>,
  overlay: ReadonlyMap<string, KeyBinding>,
): ReadonlyMap<string, KeyBinding> {
  const merged = new Map<string, KeyBinding>(base);
  for (const [id, binding] of overlay) {
    if (merged.has(id)) throw new Error(`keymap: duplicate binding id "${id}" across maps`);
    merged.set(id, binding);
  }
  return merged;
}

/** The shell's effective keymap: C1's seed + the variant toggles. */
export const PLAYER_SHELL_KEYMAP: ReadonlyMap<string, KeyBinding> = mergeKeymaps(
  DEFAULT_KEYMAP,
  VARIANT_KEYMAP,
);
