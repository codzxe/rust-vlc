// w26.b10 — lane i18n seam (contract §4.5). `useMsg` extended with this
// lane's pre-registration keys: lane keys win while they are unregistered;
// once the Lead merges them into ui/src/i18n/messages.ts both paths return
// the same string (the lane file copy is the reference). Every user-visible
// options string flows through here — no literals (doc 07 §6.5).
//
// w26.gfix-E — the catalog also carries the file-row context-menu keys
// (ui/src/i18n/keys/w26.gfixE.ts, file context menu.svg) so the same seam
// resolves them for MediaList hosts.

import { useCallback } from "react";
import { useMsg, type MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b10";
import { keys as gfixEKeys } from "../../../i18n/keys/w26.gfixE";

/** Keys owned by this lane (registered into messages.ts by the Lead). */
export type OptionsLaneKey = keyof typeof laneKeys | keyof typeof gfixEKeys;

const laneCatalog: Readonly<Record<string, string>> = { ...laneKeys, ...gfixEKeys };

/**
 * useOptionsMsg — `useMsg` widened with the w26.b10 playlist-options keys.
 * Mirrors the AppShell `useShellMsg` seam-first resolver.
 */
export function useOptionsMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | OptionsLaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}
