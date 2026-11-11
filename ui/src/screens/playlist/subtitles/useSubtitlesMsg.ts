// w26.b11 — lane i18n seam (contract §4.5). `useMsg` extended with this
// lane's pre-registration keys: lane keys win while they are unregistered;
// once the Lead merges them into ui/src/i18n/messages.ts both paths return
// the same string (the lane file copy is the reference). Every user-visible
// subtitle-flow string flows through here — no literals (doc 07 §6.5).
// Same pattern as the sibling playlist lanes (w26.b09 / w26.b10).

import { useCallback } from "react";
import { useMsg, type MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b11";

/** Keys owned by this lane (registered into messages.ts by the Lead). */
export type SubtitlesLaneKey = keyof typeof laneKeys;

const laneCatalog: Readonly<Record<string, string>> = laneKeys;

/**
 * useSubtitlesMsg — `useMsg` widened with the w26.b11 subtitle-flow keys.
 * Mirrors the usePlaylistMsg seam-first resolver.
 */
export function useSubtitlesMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | SubtitlesLaneKey): string => laneCatalog[key] ?? m(key as MessageKey),
    [m],
  );
}
