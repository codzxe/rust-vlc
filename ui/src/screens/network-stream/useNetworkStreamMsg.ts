// w26.b15 — lane i18n seam (contract §4.5) for the open-network-stream
// dialog. The frame copy ("You can enter multiple links to load.",
// "Playlist Name (optional)", …) has no catalog entry yet; the keys live in
// `ui/src/i18n/keys/w26.b15.ts` until the Lead registers them into
// `messages.ts` post-wave. This hook resolves lane keys first (identical
// copy before and after registration), falling back to the shared catalog —
// so NO user-visible string is a literal and none renders `undefined` while
// unregistered. Same pattern as the playlist/discovery/settings lanes
// (w26.b09 / w26.b12 / w26.b01).

import { useCallback } from "react";
import { useMsg, type MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b15";

/** Keys owned by this lane (registered into messages.ts by the Lead). */
export type NetworkStreamLaneKey = keyof typeof laneKeys;

const laneCatalog: Readonly<Record<string, string>> = laneKeys;

/**
 * useNetworkStreamMsg — `useMsg` extended with this lane's pre-registration
 * keys. Lane keys win while they are unregistered; once the Lead merges them
 * into the catalog both paths return the same string.
 */
export function useNetworkStreamMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | NetworkStreamLaneKey): string => laneCatalog[key] ?? m(key as MessageKey),
    [m],
  );
}
