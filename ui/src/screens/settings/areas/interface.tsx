import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as fixerKeys } from "../../../i18n/keys/w26.f03";
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
// Sections stay SCHEMA-DRIVEN (SETTINGS_AREAS, the w26.f01 seed): the
// frame rows (Skin "Use a custom skin"/"Skin file", Window "Play video
// inside main window"/"Resize window to fit video"/…, Playback Controls
// "Hide after" slider, …) render here the moment the seed carries them —
// no fabricated rows before that (config-never-invents, doc 05 §6.4).

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
type FixerKey = keyof typeof fixerKeys;
/** Pre-registration seam: b01 + b02 + f03 lane keys (see w26.f03.ts). */
const laneCatalog: Readonly<Record<string, string>> = {
  ...registryKeys,
  ...laneKeys,
  ...fixerKeys,
};

/** useInterfaceMsg — useMsg + the settings lanes' pre-registration keys. */
function useInterfaceMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey | RegistryKey | FixerKey): string =>
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
    </div>
  );
}
