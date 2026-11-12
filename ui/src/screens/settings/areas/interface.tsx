import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as fixerKeys } from "../../../i18n/keys/w26.f03";
import { keys as gfixKeys } from "../../../i18n/keys/w26.gfixB";
import { SETTINGS_AREAS } from "../generated/schema";
import { useSettingsSearch } from "../search";
import { SettingField } from "../SettingField";

// InterfaceArea — hand-authored (w26.f03, e05 F3/F7). The Figma "Settings -
// interface" frame (Settings - interface.svg) composes Skin · Window ·
// Playback Controls · Notifications and System Tray and carries NO
// "Advanced Options" bar (tools-settings §C2 + the SettingsSections
// comment) — the advanced disclosure belongs to the frames that show it
// (global, performance, …). So this screen renders its schema sections
// FLAT: every seeded interface row (advanced-flagged or not) renders in
// its section, in schema order, through SettingField — special widgets
// (SPECIAL_FIELD_WIDGETS), type controls, dirty/restart contracts and
// search filtering all stay framework-driven.
//
// w26.gfix-B (g10-P1-1): the seed now carries the frame's full inventory
// (Skin: Use a custom skin · Skin file; Window: the 7 frame rows incl. the
// relabeled "Remember window size and position"; Playback Controls: the
// "Hide after" slider). The frame's FOURTH section — "Notifications and
// System Tray" (y837) — is a HEADER ONLY (its rows sit below the exported
// fold; no [[setting]] rows exist), so it renders as the bare hand header
// below the schema sections.

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
type FixerKey = keyof typeof fixerKeys;
type GfixKey = keyof typeof gfixKeys;
/** Pre-registration seam: b01 + b02 + f03 + gfixB lane keys. */
const laneCatalog: Readonly<Record<string, string>> = {
  ...registryKeys,
  ...laneKeys,
  ...fixerKeys,
  ...gfixKeys,
};

/** useInterfaceMsg — useMsg + the settings lanes' pre-registration keys. */
function useInterfaceMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey | RegistryKey | FixerKey | GfixKey): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}

/** The interface area's schema sections (seed truth, schema order). */
const INTERFACE_SECTIONS = SETTINGS_AREAS.find((area) => area.id === "interface")?.sections ?? [];

export function InterfaceArea(): ReactElement {
  const t = useInterfaceMsg();
  const { matches } = useSettingsSearch();

  return (
    <div className="settings-sections" data-settings-area="interface">
      {INTERFACE_SECTIONS.map((section) => {
        const entries = section.entries.filter((entry) => entry.featureExposed && matches(entry));
        if (entries.length === 0) return null;
        return (
          <section key={section.id} className="settings-section" data-section={section.id}>
            <h2 className="settings-section__title">{t(section.title as MessageKey)}</h2>
            {entries.map((entry) => (
              <SettingField key={entry.key} entry={entry} />
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
