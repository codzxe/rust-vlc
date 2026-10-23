// Settings area: Advanced — card 4.E4. Every schema entry flagged
// `advanced = true` (E1's advanced-badge mechanism) folded into one screen,
// collapsed by default and expanded on demand — the doc 07 §5.4 "Advanced"
// fold the design package asks for. No option list of its own: the entries
// come from the generated schema registry (SETTINGS_ENTRIES), each rendered
// through E1's SettingField so badges, restart notices, feature gating
// (hidden + inert, doc 05 §6.4) and search behave exactly as on their home
// screens. Engine-internal entries (ui = false, screen "engine") are never
// rendered here — the generator emits no such screens and neither do we.

import { useState } from "react";
import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { SettingField } from "../SettingField";
import { SETTINGS_ENTRIES } from "../generated/schema";
import type { SettingEntry } from "../generated/schema";
import { GENERATED_AREA_IDS } from "../generated/Screens";
import { GENERATED_AREA_TITLES } from "../generated/Screens";

/** Advanced-flagged entries on UI screens (engine ui=false screens excluded). */
export function advancedEntries(): readonly SettingEntry[] {
  const uiScreens = new Set<string>(GENERATED_AREA_IDS);
  return SETTINGS_ENTRIES.filter((entry) => entry.advanced && uiScreens.has(entry.screen));
}

/** Advanced settings area — one fold, every advanced-flagged schema entry. */
export function AdvancedArea(): ReactElement {
  const m = useMsg();
  const [open, setOpen] = useState(false);
  const byScreen = new Map<string, SettingEntry[]>();
  for (const entry of advancedEntries()) {
    const list = byScreen.get(entry.screen) ?? [];
    list.push(entry);
    byScreen.set(entry.screen, list);
  }
  return (
    <div className="settings-sections" data-settings-area="advanced" data-testid="advanced-area">
      <button
        type="button"
        className="settings-screen__reset"
        aria-expanded={open}
        data-testid="advanced-fold"
        onClick={() => setOpen(!open)}
      >
        {m(open ? "settings.advanced.fold-hide" : "settings.advanced.fold-show")}
      </button>
      <p className="settings-field__badges" role="note">
        {m("settings.advanced.intro")}
      </p>
      <div hidden={!open} data-testid="advanced-fold-body">
        {[...byScreen.entries()].map(([screen, entries]) => (
          <section key={screen} className="settings-section" data-section={screen}>
            <h2 className="settings-section__title">{m(GENERATED_AREA_TITLES[screen as keyof typeof GENERATED_AREA_TITLES] as MessageKey)}</h2>
            {entries.map((entry) => (
              <SettingField key={entry.key} entry={entry} />
            ))}
          </section>
        ))}
      </div>
    </div>
  );
}
