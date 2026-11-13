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
//
// w26.k13 (j13 ruling 1): the seam hook + section loop ride the shared
// areas layer (makeAreaMsg + schemaSectionViews). w26.k12 (j12 P2-2): no
// search filter and no hand-row keywords — the shell swaps in cross-area
// results while a query is live (an area-side filter could never execute),
// and hand rows stay invisible to that search by design (it walks schema
// entries only); the Clear-history row always renders in its section.

import { useId, useState } from "react";
import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as i13Keys } from "../../../i18n/keys/w26.i13";
import { SettingField } from "../SettingField";
import { SettingInfoDot } from "../SettingField";
import { makeAreaMsg } from "./shared";
import { schemaSectionViews } from "./shared";

/** useAreaMsg — useMsg + this lane's pre-registration keys (w26.i13 hints). */
const useAreaMsg = makeAreaMsg(i13Keys);

/** ⓘ hint msgids per schema row (w26.i13, h08 #3 — every privacy row). */
const PRIVACY_HINTS: Readonly<Record<string, keyof typeof i13Keys>> = {
  "privacy.save-recent-items": "settings.hint.privacy.save-recent-items",
  "privacy.remember-playback-position": "settings.hint.privacy.remember-playback-position",
  "privacy.never-remember-matching": "settings.hint.privacy.never-remember-matching",
  "privacy.fetch-metadata-online": "settings.hint.privacy.fetch-metadata-online",
  "privacy.submit-lastfm": "settings.hint.privacy.submit-lastfm",
  "privacy.lastfm-username": "settings.hint.privacy.lastfm-username",
  "privacy.lastfm-password": "settings.hint.privacy.lastfm-password",
  "privacy.lastfm-submission-url": "settings.hint.privacy.lastfm-submission-url",
};

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
  const hintCaptionId = useId();
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
            controlsId={hintCaptionId}
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
      {hintOpen ? (
        <p id={hintCaptionId} role="note" className="settings-field__hinttext">
          {m("settings.hint.privacy.clear-history")}
        </p>
      ) : null}
    </div>
  );
}

/**
 * Privacy settings area (route /settings/privacy; App group) — the frame's
 * History + Network sections from the schema seed, hand-rendered with per-row
 * ⓘ hints (h08 #3), plus the Clear-history action row at History's end.
 */
export function PrivacyArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="privacy">
      {schemaSectionViews("privacy").map(({ section, visible }) => {
        // History's action row (frame C3 y244) closes the section in frame
        // order; hand rows always render in the normal flow (w26.k12).
        const showClearRow = section.id === "history";
        if (visible.length === 0 && !showClearRow) return null;
        return (
          <section key={section.id} className="settings-section" data-section={section.id}>
            <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
            {visible.map((entry) => (
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
