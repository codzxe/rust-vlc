// w26.c3 — lane i18n seam (contract §4.5, the w26.c1 useEqMsg pattern).
// `useMsg` widened with this lane's pre-registration keys plus the tool-card
// toggle keys owned by w26.a18: lane keys win while they are unregistered;
// once the Lead merges them into ui/src/i18n/messages.ts both paths return
// the same string. Every user-visible visualizer string flows through here —
// no literals (doc 07 §6.5).

import { useCallback } from "react";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import { keys as visualizerKeys } from "../../../../i18n/keys/w26.c3";
import { keys as toolsKeys } from "../../../../i18n/keys/w26.a18";
import type { VisualizerLabelKey } from "./types";

/** The lane catalog: this lane's keys over the reused a18 tool-card keys. */
const laneCatalog: Readonly<Record<string, string>> = {
  ...toolsKeys,
  ...visualizerKeys,
};

/**
 * useVisualizerMsg — `useMsg` widened with the w26.c3 visualizer keys (and
 * the reused w26.a18 tool-card toggle keys). Seam-first: the central catalog
 * wins once the Lead registers the keys, the lane copy is the fallback
 * reference.
 */
export function useVisualizerMsg(): (key: VisualizerLabelKey) => string {
  const m = useMsg();
  return useCallback(
    (key: VisualizerLabelKey): string =>
      m(key as MessageKey) ?? laneCatalog[key] ?? key,
    [m],
  );
}
