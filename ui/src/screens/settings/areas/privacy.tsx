// Settings area: Privacy — "Privacy Settings" (Settings - privacy.svg,
// analysis tools-settings.md §C3; w26.g11-P1-1 composition fix).
//
// w26.gfix-B COMPOSITION (frame truth; the schema seed carries every schema
//   row — the generated section data stays the source of truth):
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
//
// w26.i13 (h08 #3): the privacy frame hangs a ⓘ circle on EVERY row — the
// 8 schema rows plus the Clear-history ACTION row (circle extraction: cy
// 130/172/216/260 + 361/403/447/493/539, all 9 rows) — so this module
// hand-renders its sections (the InterfaceArea precedent) to pass per-row
// `hint` props through SettingField, and ClearHistoryRow carries the shared
// SettingInfoDot grammar (i12) with this lane's hint copy. The screen stays
// schema-driven: sections/entries/order come from the generated seed.

import { useState } from "react";
import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as i13Keys } from "../../../i18n/keys/w26.i13";
import type { SettingEntry } from "../generated/schema";
import { schemaSectionsOf } from "../SettingsSections";
import { SettingField } from "../SettingField";
import { SettingInfoDot } from "../SettingField";
import { useSettingsSearch } from "../search";

type I13Key = keyof typeof i13Keys;
/** Pre-registration seam (see w26.i13.ts) — the hint copy for this area. */
const laneCatalog: Readonly<Record<string, string>> = i13Keys;

/** useAreaMsg — useMsg + this lane's pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | I13Key): string => m(key as MessageKey) ?? laneCatalog[key];
}

/** ⓘ hint msgids per schema row (w26.i13, h08 #3 — every privacy row). */
const PRIVACY_HINTS: Readonly<Record<string, I13Key>> = {
  "privacy.save-recent-items": "settings.hint.privacy.save-recent-items",
  "privacy.remember-playback-position": "settings.hint.privacy.remember-playback-position",
  "privacy.never-remember-matching": "settings.hint.privacy.never-remember-matching",
  "privacy.fetch-metadata-online": "settings.hint.privacy.fetch-metadata-online",
  "privacy.submit-lastfm": "settings.hint.privacy.submit-lastfm",
  "privacy.lastfm-username": "settings.hint.privacy.lastfm-username",
  "privacy.lastfm-password": "settings.hint.privacy.lastfm-password",
  "privacy.lastfm-submission-url": "settings.hint.privacy.lastfm-submission-url",
};

/** The hand row's search terms (SettingsSections' SettingRowNode contract). */
const CLEAR_HISTORY_KEYWORDS = ["Clear history now", "Clear"] as const;

/**
 * The "Clear history now" action row (frame C3 y244 — with its ⓘ circle at
 * cy 260, the only frame ⓘ on an ACTION row). HONEST SURFACE RULE: no
 * playback-history store or clearing command exists in the catalog yet
 * (recently-played is the Phase-4 library feature) — the frame-verbatim row
 * renders with its Clear button DISABLED WITH THE HONEST REASON (visible,
 * inert, never a dead control that looks alive). It goes live the moment the
 * history seam lands.
 */
export function ClearHistoryRow(): ReactElement {
  const m = useAreaMsg();
  const [hintOpen, setHintOpen] = useState(false);
  const reason = m("settings.privacy.clear-unavailable");
  return (
    <div className="settings-field" data-kind="action" data-privacy-action="clear-history">
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{m("settings.privacy.clear-history")}</span>
          <SettingInfoDot
            note={m("settings.hint.privacy.clear-history")}
            label={m("settings.field.more-info")}
            expanded={hintOpen}
            onToggle={() => setHintOpen((open) => !open)}
          />
        </div>
        <div className="settings-field__control">
          <button
            type="button"
            className="settings-action"
            disabled
            title={reason}
            aria-label={`${m("settings.privacy.clear")} — ${reason}`}
          >
            {m("settings.privacy.clear")}
          </button>
        </div>
      </div>
      {hintOpen ? <p className="settings-field__hinttext">{m("settings.hint.privacy.clear-history")}</p> : null}
    </div>
  );
}

/**
 * Privacy settings area (route /settings/privacy; App group) — the frame's
 * History + Network sections from the schema seed, hand-rendered with per-row
 * ⓘ hints (h08 #3), plus the Clear-history action row at History's end.
 */
export function PrivacyArea(): ReactElement {
  const m = useAreaMsg();
  const { query, matches } = useSettingsSearch();
  const needle = query.trim().toLowerCase();
  const clearRowMatches =
    needle === "" || CLEAR_HISTORY_KEYWORDS.some((keyword) => keyword.toLowerCase().includes(needle));

  return (
    <div className="settings-sections" data-settings-area="privacy">
      {schemaSectionsOf("privacy").map((section) => {
        // Pure-schema sections (the Clear-history hand row is rendered by
        // this module, not spliced into the section data): narrow the row
        // union so the schema-entry fields type-check below.
        const entries = section.entries
          .filter((row): row is SettingEntry => "key" in row)
          .filter((entry) => entry.featureExposed && matches(entry));
        // History's action row (frame C3 y244) rides the section's own
        // visibility while a query is live (its keywords are its filter).
        const showClearRow = section.id === "history" && clearRowMatches;
        if (entries.length === 0 && !showClearRow) return null;
        return (
          <section key={section.id} className="settings-section" data-section={section.id}>
            <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
            {entries.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={PRIVACY_HINTS[entry.key]} />
            ))}
            {showClearRow ? (
              <div data-setting-row="clear-history">
                <ClearHistoryRow />
              </div>
            ) : null}
          </section>
        );
      })}
    </div>
  );
}
