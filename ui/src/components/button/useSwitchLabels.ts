// useSwitchLabels — the switch core's i18n seam (contract §4.5, w26.k01
// j01-4). The ON/OFF micro labels are user-visible copy (sighted users see
// them; aria-hidden only removes them from the a11y tree), so they resolve
// through useMsg() — NOT through literals. The msgids live in
// `ui/src/i18n/keys/w26.k01.ts` until the Lead registers them into
// `messages.ts` post-wave; lane keys win while unregistered, the shared
// catalog wins after (identical en-US copy either way). Same seam-first
// pattern as useNetworkStreamMsg (w26.b15) / useEqMsg (w26.f05).

import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.k01";

/** The localized ON/OFF micro-label pair (Toggle.svg frame text). */
export function useSwitchLabels(): { on: string; off: string } {
  const m = useMsg();
  return {
    on: m("switch.on" as MessageKey) || laneKeys["switch.on"],
    off: m("switch.off" as MessageKey) || laneKeys["switch.off"],
  };
}
