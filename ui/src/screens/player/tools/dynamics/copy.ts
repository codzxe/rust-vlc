// w26.c2 — seam-first copy resolver for the dynamics tools (the w26.a17
// customize-panel pattern). Central catalog first (useMsg + MessageKey),
// then the lane's pre-registration key files (w26.c2 — this lane's rows;
// w26.a18 — the tools-shell toggle labels, reused), then the raw key. Copy
// renders identically before and after the Lead registers the keys into
// ui/src/i18n/messages.ts.

import { useCallback } from "react";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import { keys as dialogsLaneKeys } from "../../../../i18n/keys/w26.a18";
import { keys as laneKeys } from "../../../../i18n/keys/w26.c2";
import type { DynamicsLabelKey } from "./types";

/** Pre-registration fallbacks, merged (no key is defined twice — the toggle
 * labels belong to w26.a18, the row/staged copy to this lane). */
const FALLBACKS: Readonly<Record<string, string>> = {
  ...dialogsLaneKeys,
  ...laneKeys,
};

/** Resolves a dynamics label key: catalog → lane pre-registration → key. */
export function useDynamicsMsg(): (key: DynamicsLabelKey) => string {
  const m = useMsg();
  return useCallback(
    (key: DynamicsLabelKey): string =>
      m(key as MessageKey) ?? FALLBACKS[key] ?? key,
    [m],
  );
}
