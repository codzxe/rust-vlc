// w26.b09 — lane i18n seam (contract §4.5) for the playlist core screens.
//
// The /playlist library page and the create-playlist dialog render copy that
// has no catalog entry yet; the keys live in `ui/src/i18n/keys/w26.b09.ts`
// until the Lead registers them into `messages.ts` post-wave. This hook
// resolves lane keys first (identical copy before and after registration),
// falling back to the shared catalog — so NO user-visible string is a
// literal and none renders `undefined` while unregistered. Same pattern as
// the discovery/settings lanes (w26.b12 / w26.b01).
//
// w26.f06 — the lane catalog now also carries this wave's playlist-surface
// keys (`ui/src/i18n/keys/w26.f06.ts`: the opened-playlist browse view's
// back/empty/copy and the toolbar's Add-subtitles action), same seam-first
// grammar.

import { useCallback } from "react";
import { useMsg, type MessageKey } from "../../i18n/messages";
import { keys as b09Keys } from "../../i18n/keys/w26.b09";
import { keys as f06Keys } from "../../i18n/keys/w26.f06";

/** Keys owned by this lane (w26.b09 + the w26.f06 playlist surfaces). */
export type PlaylistLaneKey = keyof typeof b09Keys | keyof typeof f06Keys;

const laneCatalog: Readonly<Record<string, string>> = { ...b09Keys, ...f06Keys };

/**
 * usePlaylistMsg — `useMsg` extended with the playlist lane's
 * pre-registration keys. Lane keys win while they are unregistered; once the
 * Lead merges them into the catalog both paths return the same string.
 */
export function usePlaylistMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | PlaylistLaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}
