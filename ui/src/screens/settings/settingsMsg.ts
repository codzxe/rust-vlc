import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as b01Keys } from "../../i18n/keys/w26.b01";
import { keys as b02Keys } from "../../i18n/keys/w26.b02";
import { keys as f01Keys } from "../../i18n/keys/w26.f01";
import { keys as f02Keys } from "../../i18n/keys/w26.f02";
import { keys as f03Keys } from "../../i18n/keys/w26.f03";
import { keys as gfixKeys } from "../../i18n/keys/w26.gfixB";
import { keys as i13Keys } from "../../i18n/keys/w26.i13";
import { keys as i14Keys } from "../../i18n/keys/w26.i14";
import { keys as k13Keys } from "../../i18n/keys/w26.k13";

// settingsMsg.ts — w26.k12 (j12 P3-5): the settings shell's ONE lane-seam
// resolver. The four shell files used to carry four private copies
// (SettingsScreen `useSettingsMsg`, SettingField `useFieldMsg`,
// SettingsSections `useSectionsMsg`, SettingWidgets `useWidgetMsg`) with TWO
// different precedence orders — the shell resolved the f03 retitles
// lane-first (they must beat the registered catalog until the Lead carries
// them into messages.ts), while the rows/widgets/sections copies resolved
// catalog-first, so the next retitle registered through a lane file would
// have rendered different copy in the shell vs the rows. One module, one
// documented precedence now; the areas' ten copies are a separate,
// Lead-owned post-wave consolidation (j13 ruling 1) and stay untouched.
//
// PRECEDENCE (single rule, all four consumers):
//   1. the lane catalog below — every settings-family lane key file,
//      spread in wave order with the retitle-carrying file (f03) LAST so a
//      retitle beats any earlier lane value for the same key;
//   2. the registered catalog (useMsg);
//   3. "" for keys resolved by neither (the established degradation: a
//      catalog gap reads as empty and can never match in search).
// While a key is unregistered this renders the lane value (pre-registration
// seam, contract §4.5); once the Lead registers it both paths return the
// same literal, so the precedence is invisible post-registration. Verified
// at w26.k12 time: zero lane keys differ from an already-registered catalog
// value (script over all eight key files vs messages.ts), so the merge is a
// zero-visible-change refactor today.

export type SettingsMsgKey =
  | MessageKey
  | keyof typeof b01Keys
  | keyof typeof b02Keys
  | keyof typeof f01Keys
  | keyof typeof f02Keys
  | keyof typeof f03Keys
  | keyof typeof gfixKeys
  | keyof typeof i13Keys
  | keyof typeof i14Keys
  | keyof typeof k13Keys;

/** The merged pre-registration catalog (exported for membership probes). */
export const settingsLaneCatalog: Readonly<Record<string, string>> = {
  ...b02Keys, // b02 registry/seed (wave B)
  ...b01Keys, // b01 shell (wave B)
  ...f01Keys, // f01 seed row labels (wave F)
  ...f02Keys, // f02 anatomy (wave F)
  ...gfixKeys, // gfix-B composition seed (wave G)
  ...i13Keys, // i13 hints (wave I)
  ...i14Keys, // i14 video enums + hints (wave I)
  ...k13Keys, // k13 performance hints (wave K, j13 P1-1 — additive)
  ...f03Keys, // f03 RETITLES last — a retitle beats every earlier lane value
};

/**
 * useSettingsMsg — `useMsg` extended with the settings family's
 * pre-registration lane keys (the a10 `useShellMsg` pattern, now shared).
 */
export function useSettingsMsg(): (key: SettingsMsgKey) => string {
  const m = useMsg();
  return (key: SettingsMsgKey): string =>
    settingsLaneCatalog[key] ?? m(key as MessageKey) ?? "";
}
