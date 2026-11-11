// w26.c4 — seam-first copy resolver for the video tools (the w26.a17/c2
// pattern). Central catalog first (useMsg + MessageKey), then the lane's
// pre-registration key files (w26.c4 — this lane's rows; w26.a18 — the
// tools-shell toggle labels, reused), then the raw key. Copy renders
// identically before and after the Lead registers the keys into
// ui/src/i18n/messages.ts.

import { useCallback } from "react";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import { keys as dialogsLaneKeys } from "../../../../i18n/keys/w26.a18";
import { keys as laneKeys } from "../../../../i18n/keys/w26.c4";
import type { VideoLabelKey } from "./types";

/** Pre-registration fallbacks, merged (no key is defined twice — the toggle
 * labels belong to w26.a18, the row/staged copy to this lane). */
const FALLBACKS: Readonly<Record<string, string>> = {
  ...dialogsLaneKeys,
  ...laneKeys,
};

/** Resolves a video label key: catalog → lane pre-registration → key. */
export function useVideoMsg(): (key: VideoLabelKey) => string {
  const m = useMsg();
  return useCallback(
    (key: VideoLabelKey): string =>
      m(key as MessageKey) ?? FALLBACKS[key] ?? key,
    [m],
  );
}
