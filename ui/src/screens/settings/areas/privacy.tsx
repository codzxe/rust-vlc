// Settings area: Privacy — "Privacy Settings" (Settings - privacy.svg,
// analysis tools-settings.md §C3; w26.g11-P1-1 composition fix).
//
// w26.gfix-B COMPOSITION (frame truth; the schema seed carries every schema
// row — the generated section data stays the source of truth):
//   History (y83) — Save recently played items ON · Remember the playback
//     position ON · Never remember items matching (input) · Clear history
//     now → Clear 100×32 rx6 #191919 @x924 (y244) — the ACTION row at the
//     section's end, spliced in by this module (no [[setting]] key can carry
//     an action; g11-P1-1).
//   Network (y314) — Fetch titles and cover art from the internet OFF ·
//     Submit played tracks to last.fm OFF · Last.fm username/password/
//     submission URL (inputs).
// The former "Data & privacy" telemetry section left the screen with the
// w26.gfix-B re-seed (g11-P2-1): its rows carry no frame anywhere in the
// Settings family and stay CLI/config-only (ui = false in
// schemas/settings.toml).

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as fixerKeys } from "../../../i18n/keys/w26.gfixB";
import { SettingsSections } from "../SettingsSections";
import { schemaSectionsOf } from "../SettingsSections";
import { withRows } from "../SettingsSections";

type FixerKey = keyof typeof fixerKeys;
/** Pre-registration seam (see w26.gfixB.ts). */
const laneCatalog: Readonly<Record<string, string>> = fixerKeys;

/** useAreaMsg — useMsg + this lane's pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | FixerKey): string => m(key as MessageKey) ?? laneCatalog[key];
}

/**
 * The "Clear history now" action row (frame C3 y244). HONEST SURFACE RULE:
 * no playback-history store or clearing command exists in the catalog yet
 * (recently-played is the Phase-4 library feature) — the frame-verbatim row
 * renders with its Clear button DISABLED WITH THE HONEST REASON (visible,
 * inert, never a dead control that looks alive). It goes live the moment the
 * history seam lands.
 */
export function ClearHistoryRow(): ReactElement {
  const m = useAreaMsg();
  const reason = m("settings.privacy.clear-unavailable");
  return (
    <div className="settings-field" data-kind="action" data-privacy-action="clear-history">
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{m("settings.privacy.clear-history")}</span>
        </div>
        <div className="settings-field__control">
          <button type="button" className="settings-action" disabled title={reason} aria-label={`${m("settings.privacy.clear")} — ${reason}`}>
            {m("settings.privacy.clear")}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Privacy settings area (route /settings/privacy; App group) — the frame's
 * History + Network sections from the schema seed + the Clear-history action
 * row.
 */
export function PrivacyArea(): ReactElement {
  return (
    <SettingsSections
      area="privacy"
      sections={withRows(schemaSectionsOf("privacy"), "history", [
        {
          row: {
            rowId: "clear-history",
            keywords: ["Clear history now", "Clear"],
            node: <ClearHistoryRow />,
          },
        },
      ])}
    />
  );
}
