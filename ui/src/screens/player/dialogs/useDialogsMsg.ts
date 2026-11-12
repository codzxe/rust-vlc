// w26.k09 — lane i18n seam (contract §4.5) for the player dialogs the j09
// review found bypassing useMsg: Bookmarks + Go-to-Time rendered hardcoded
// English COPY constants with no keys file behind them, and the tools shell
// duplicated keys already registered central. The keys live in
// `ui/src/i18n/keys/w26.k09.ts` until the Lead registers them into
// `messages.ts` post-wave; useDialogsMsg resolves central-first with the
// lane catalog as fallback (the useToolsMsg / useNetworkStreamMsg pattern),
// so the copy renders identically before and after registration and
// pseudo-locale covers both dialogs once the keys are central.
//
// The derived COPY constants below consolidate out of the component files
// (j09 P3: the react-refresh only-export-components pattern on COPY
// exports) while keeping the frozen public surface — index.ts re-exports
// the same names and the tests read them as the string source. They derive
// from the SAME lane keys the components resolve (one source of truth, no
// drift); the components render through useDialogsMsg, never these objects.
// The tools shell's three strings need no lane resolver — its player.tools.*
// keys are already central (messages.ts), so ToolsDialogShell calls useMsg
// directly; TOOLS_DIALOG_COPY derives from the w26.a18 file that recorded
// them.

import { useCallback } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.k09";
import { keys as shellKeys } from "../../../i18n/keys/w26.a18";

/** Keys owned by this lane (registered into messages.ts by the Lead). */
export type DialogsLaneKey = keyof typeof laneKeys;

const laneCatalog: Readonly<Record<string, string>> = laneKeys;

/**
 * useDialogsMsg — `useMsg` extended with this lane's pre-registration keys.
 * The central catalog wins once the Lead registers the keys; the lane copy
 * is the fallback reference (same string either way).
 */
export function useDialogsMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | DialogsLaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}

/**
 * The Bookmarks dialog's copy, derived from the w26.k09 lane keys. Test +
 * back-compat reference only (the frozen index.ts surface); the component
 * renders through {@link useDialogsMsg}.
 */
export const BOOKMARKS_COPY = {
  title: laneKeys["player.dialog.bookmarks.title"],
  subtitle: laneKeys["player.dialog.bookmarks.subtitle"],
  label: laneKeys["player.dialog.bookmarks.label"],
  placeholder: laneKeys["player.dialog.bookmarks.placeholder"],
  add: laneKeys["player.dialog.bookmarks.add"],
  /** Row accessible name — time AND label (j09 P3: same-time rows stay
   * distinguishable to screen readers). */
  jump: (time: string, label: string): string =>
    laneKeys["player.dialog.bookmarks.jump"]
      .replace("{time}", time)
      .replace("{label}", label),
  deleteBookmark: (time: string): string =>
    laneKeys["player.dialog.bookmarks.delete"].replace("{time}", time),
  unnamed: laneKeys["player.dialog.bookmarks.unnamed"],
  empty: laneKeys["player.dialog.bookmarks.empty"],
  close: laneKeys["player.dialog.bookmarks.close"],
  loadFailed: laneKeys["player.dialog.bookmarks.load-failed"],
  addFailed: laneKeys["player.dialog.bookmarks.add-failed"],
  deleteFailed: laneKeys["player.dialog.bookmarks.delete-failed"],
} as const;

/**
 * The Go-to-Time dialog's copy, derived from the w26.k09 lane keys. Test +
 * back-compat reference only; the component renders through
 * {@link useDialogsMsg}.
 */
export const GOTO_COPY = {
  title: laneKeys["player.dialog.goto.title"],
  subtitle: laneKeys["player.dialog.goto.subtitle"],
  label: laneKeys["player.dialog.goto.label"],
  placeholder: laneKeys["player.dialog.goto.placeholder"],
  invalid: laneKeys["player.dialog.goto.invalid"],
  beyondEnd: (duration: string): string =>
    laneKeys["player.dialog.goto.beyond-end"].replace("{duration}", duration),
  action: laneKeys["player.dialog.goto.action"],
  cancel: laneKeys["player.dialog.goto.cancel"],
  close: laneKeys["player.dialog.goto.close"],
} as const;

/**
 * The tools shell's copy, derived from the w26.a18 lane keys that recorded
 * the (already central) player.tools.* strings. Test + back-compat
 * reference only; ToolsDialogShell renders through useMsg directly.
 */
export const TOOLS_DIALOG_COPY = {
  /** Collapsed card toggle label (frame: "Expand Settings"). */
  expand: shellKeys["player.tools.expand-settings"],
  /** Expanded card toggle label (frame: "Collapse Settings"). */
  collapse: shellKeys["player.tools.collapse-settings"],
  /** Accessible name of the header close ✕. */
  close: shellKeys["player.tools.close"],
} as const;
