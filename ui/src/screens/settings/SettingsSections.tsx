import { useState } from "react";
import type { ReactElement } from "react";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../i18n/keys/w26.b02";
import { Icon } from "../../components/icon/Icon";
import type { SettingEntry } from "./generated/schema";
import { SettingField } from "./SettingField";
import { useSettingsSearch } from "./search";

export interface GeneratedSection {
  readonly id: string;
  readonly title: MessageKey;
  readonly entries: readonly SettingEntry[];
}

// SettingsSections — the generated screens' content half, in the W26 Figma
// grammar (tools-settings.md §D2): section headers + label-left/control-right
// rows, closed by the IN-FLOW "Advanced Options" disclosure (Settings -
// global.svg Frame 265: 784×38 r2 bar, label + › chevron; global-1.svg is
// the expanded state — ∨ chevron, advanced rows render BELOW the bar, no
// extra section header). Schema entries flagged `advanced` are segregated
// behind that disclosure instead of carrying badges in the normal flow.

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
/** Pre-registration seam: b01 + b02 (registry/seed) lane keys (see w26.b02). */
const laneCatalog: Readonly<Record<string, string>> = { ...registryKeys, ...laneKeys };

/** useSectionsMsg — useMsg + the settings lanes' pre-registration keys. */
function useSectionsMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey | RegistryKey): string => m(key as MessageKey) ?? laneCatalog[key];
}

/** Split a section's entries into (normal, advanced), search + gate filtered. */
function partition(
  entries: readonly SettingEntry[],
  matches: (entry: SettingEntry) => boolean,
): { normal: readonly SettingEntry[]; advanced: readonly SettingEntry[] } {
  const visible = entries.filter((entry) => entry.featureExposed && matches(entry));
  return {
    normal: visible.filter((entry) => !entry.advanced),
    advanced: visible.filter((entry) => entry.advanced),
  };
}

/**
 * The "Advanced Options" disclosure — in flow at the content column's
 * bottom (NOT a pinned footer): collapsed shows the › chevron, expanded the
 * ∨ chevron and renders the advanced rows below the bar.
 */
function AdvancedDisclosure({
  open,
  onToggle,
  children,
}: {
  open: boolean;
  onToggle: () => void;
  children: ReactElement;
}) {
  const t = useSectionsMsg();
  return (
    <div className="settings-advanced">
      <button
        type="button"
        className="settings-advanced__bar"
        aria-expanded={open}
        aria-controls="settings-advanced-rows"
        onClick={onToggle}
      >
        <span className="settings-advanced__label">{t("settings.advanced-options")}</span>
        <Icon
          name={open ? "chevron-down" : "chevron-right"}
          size={14}
          className="settings-advanced__chevron"
        />
      </button>
      {open ? (
        <div className="settings-advanced__rows" id="settings-advanced-rows">
          {children}
        </div>
      ) : null}
    </div>
  );
}

/** Advanced rows — one SettingField per entry, flat under the bar (global-1). */
function AdvancedRows({ entries }: { entries: readonly SettingEntry[] }) {
  return (
    <>
      {entries.map((entry) => (
        <SettingField key={entry.key} entry={entry} />
      ))}
    </>
  );
}

export function SettingsSections({ area, sections }: { area: string; sections: readonly GeneratedSection[] }) {
  const t = useSectionsMsg();
  const { matches } = useSettingsSearch();
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const split = sections.map((section) => ({ section, ...partition(section.entries, matches) }));
  const visible = split.filter(({ normal }) => normal.length > 0);
  // Advanced rows keep schema order across sections (flat list, no headers).
  const advancedEntries = split.flatMap(({ advanced }) => advanced);

  return (
    <div className="settings-sections" data-settings-area={area}>
      {visible.map(({ section, normal }) => (
        <section key={section.id} className="settings-section" data-section={section.id}>
          <h2 className="settings-section__title">{t(section.title as MessageKey | LaneKey | RegistryKey)}</h2>
          {normal.map((entry) => (
            <SettingField key={entry.key} entry={entry} />
          ))}
        </section>
      ))}
      {/* The frames show the bar only where advanced content exists
       * (global.svg carries it; interface/controls.svg do not) — so the
       * disclosure renders only when the schema actually flags advanced
       * entries for this area. */}
      {advancedEntries.length > 0 ? (
        <AdvancedDisclosure open={advancedOpen} onToggle={() => setAdvancedOpen((v) => !v)}>
          <AdvancedRows entries={advancedEntries} />
        </AdvancedDisclosure>
      ) : null}
    </div>
  );
}
