// w26.k06 (J06-9, P3) — ONE lane-key resolver for the player family.
//
// Contract §4.5: user-visible lane copy resolves through `useMsg()` FIRST
// (the Lead registers lane keys in messages.ts post-wave) and falls back to
// the lane catalog while unregistered — rendering identically before/after
// the merge. The wave-J review counted four divergent copies of this
// resolver under the player screen (PlayerScreen's triple-catalog chain,
// PlaylistsDock's `tm`, PlaylistBrowse's local `useLaneMsg`, StageOverlays'
// HomeOverlay `tm`); this module is the single shared shape.

import { useMsg, type MessageKey } from "../../i18n/messages";

/**
 * Lane-key resolver bound to one catalog: `(key) => useMsg(key) ?? catalog[key]`.
 * The returned function is fresh per render (it closes over the current
 * `useMsg`) — call it during render, not in effect deps.
 */
export function useLaneMsg<T extends Record<string, string>>(
  catalog: T,
): (key: MessageKey | (keyof T & string)) => string {
  const m = useMsg();
  return (key: MessageKey | (keyof T & string)): string =>
    m(key as MessageKey) ?? catalog[key as keyof T];
}
