import type { ReactElement } from "react";
import type { MessageKey } from "../../../i18n/messages";
import { keys as shellKeys } from "../../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as fixerKeys } from "../../../i18n/keys/w26.f03";
import { keys as gfixKeys } from "../../../i18n/keys/w26.gfixB";
import { keys as i13Keys } from "../../../i18n/keys/w26.i13";
import { SettingField } from "../SettingField";
import { makeAreaMsg } from "./shared";
import { schemaSectionViews } from "./shared";

// InterfaceArea — hand-authored (w26.f03, e05 F3/F7). The Figma "Settings -
// interface" frame (Settings - interface.svg) composes Skin · Window ·
// Playback Controls and Notifications and System Tray and carries NO
// "Advanced Options" bar (tools-settings §C2 + the SettingsSections
// comment) — the advanced disclosure belongs to the frames that show it
// (global, performance, …). So this screen renders its schema sections
// FLAT: every seeded interface row (advanced-flagged or not) renders in
// its section, in schema order, through SettingField — special widgets
// (SPECIAL_FIELD_WIDGETS), type controls, dirty/restart contracts all stay
// framework-driven.
//
// w26.gfix-B (g10-P1-1): the seed now carries the frame's full inventory
// (Skin: Use a custom skin · Skin file; Window: the 7 frame rows incl. the
// relabeled "Remember window size and position"; Playback Controls: the
// "Hide after" slider). The frame's FOURTH section — "Notifications and
// System Tray" (y837) — is a HEADER ONLY (its rows sit below the exported
// fold; no [[setting]] rows exist), so it renders as the bare hand header
// below the schema sections.
//
// w26.i13 (h08 #3): the frame hangs ⓘ circles on 11 of the 13 rows
// (Settings - interface.svg — circle extraction: Skin×2, Window 6 of 7,
// Playback Controls 3 of 4). The two honest absences: "Remember window size
// and position" (frame cy 367 carries NO circle) and the "Hide after"
// slider label (x240–305, y728 — the next glyph is the "0 s" scale mark;
// the row's ⓘ in the frame at cy 783 belongs to "Show controls in
// fullscreen" below it).
//
// w26.k13 (j13 ruling 1): the seam hook + section loop ride the shared
// areas layer (makeAreaMsg + schemaSectionViews). w26.k12 (j12 P2-2): no
// search filter — the shell swaps in cross-area results while a query is
// live, so an area-side filter could never execute.

/** useInterfaceMsg — useMsg + the interface lanes' pre-registration keys. */
const useInterfaceMsg = makeAreaMsg(shellKeys, registryKeys, fixerKeys, gfixKeys, i13Keys);

/** ⓘ hint msgids per row (w26.i13, h08 #3) — frame-verbatim presence: the
 * 11 rows the interface frame carries a circle for; the 2 absent rows
 * (remember-window-geometry, hide-controls-after) render no ⓘ, honestly. */
const INTERFACE_HINTS: Readonly<Record<string, keyof typeof i13Keys>> = {
  "interface.use-custom-skin": "settings.hint.interface.use-custom-skin",
  "interface.skin-file": "settings.hint.interface.skin-file",
  "interface.play-video-in-main-window": "settings.hint.interface.play-video-in-main-window",
  "interface.resize-to-fit-video": "settings.hint.interface.resize-to-fit-video",
  "interface.keep-above": "settings.hint.interface.keep-above",
  "interface.window-decorations": "settings.hint.interface.window-decorations",
  "interface.filename-in-title-bar": "settings.hint.interface.filename-in-title-bar",
  "interface.open-fullscreen-on": "settings.hint.interface.open-fullscreen-on",
  "interface.start-in-compact-mode": "settings.hint.interface.start-in-compact-mode",
  "interface.auto-hide-controls": "settings.hint.interface.auto-hide-controls",
  "interface.show-controls-in-fullscreen": "settings.hint.interface.show-controls-in-fullscreen",
};

export function InterfaceArea(): ReactElement {
  const t = useInterfaceMsg();

  return (
    <div className="settings-sections" data-settings-area="interface">
      {schemaSectionViews("interface").map(({ section, visible }) => {
        if (visible.length === 0) return null;
        return (
          <section key={section.id} className="settings-section" data-section={section.id}>
            <h2 className="settings-section__title">{t(section.title as MessageKey)}</h2>
            {visible.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={INTERFACE_HINTS[entry.key]} />
            ))}
          </section>
        );
      })}
      {/* Frame C2 y837: header-only section — the frame exports no rows
       * below it. Honest absence: no fabricated rows, the header stays to
       * match the frame's section inventory. */}
      <section className="settings-section" data-section="notifications">
        <h2 className="settings-section__title">{t("settings.section.interface.notifications")}</h2>
      </section>
    </div>
  );
}
