// w26.c1 — lane i18n seam (contract §4.5, the w26.b10 useOptionsMsg
// pattern). `useMsg` widened with this lane's pre-registration keys plus the
// tool-card toggle keys owned by w26.a18 and the A2 panel header keys owned
// by w26.f05: lane keys win while they are unregistered; once the Lead
// merges them into ui/src/i18n/messages.ts both paths return the same
// string. Every user-visible equalizer string flows through here — no
// literals (doc 07 §6.5).

import { useCallback } from "react";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import { keys as eqKeys } from "../../../../i18n/keys/w26.c1";
import { keys as toolsKeys } from "../../../../i18n/keys/w26.a18";
import { keys as panelKeys } from "../../../../i18n/keys/w26.f05";

/** The lane catalog: this lane's keys over the reused a18/f05 keys. */
const laneCatalog: Readonly<Record<string, string>> = {
  ...toolsKeys,
  ...eqKeys,
  ...panelKeys,
};

/** Keys resolvable by `useEqMsg` (own lane keys + reused a18/f05 keys). */
export type EqMessageKey =
  | keyof typeof eqKeys
  | keyof typeof toolsKeys
  | keyof typeof panelKeys;

/**
 * useEqMsg — `useMsg` widened with the w26.c1 equalizer keys (and the reused
 * w26.a18 tool-card + w26.f05 panel-header keys). Seam-first: the central
 * catalog wins once the Lead registers the keys, the lane copy is the
 * fallback reference.
 */
export function useEqMsg() {
  const m = useMsg();
  return useCallback(
    // w26.k10 (j10 P3-1): the final `?? key` fallback — the siblings
    // (copy.ts / useVisualizerMsg.ts) carry it; an unknown key must render
    // the key, never `undefined`.
    (key: MessageKey | EqMessageKey): string =>
      m(key as MessageKey) ?? laneCatalog[key] ?? key,
    [m],
  );
}
