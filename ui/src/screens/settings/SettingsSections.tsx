import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import type { SettingEntry } from "./generated/schema";
import { SettingField } from "./SettingField";
import { useSettingsSearch } from "./search";

export interface GeneratedSection {
  readonly id: string;
  readonly title: MessageKey;
  readonly entries: readonly SettingEntry[];
}

/**
 * SettingsSections — framework half of the generated screens (card 4.E1):
 * the generated Screens.tsx statically binds one <SettingsSections> per
 * settings area; this component renders the section headings and fields,
 * honoring the active search (sections with no matching fields collapse).
 */
export function SettingsSections({ area, sections }: { area: string; sections: readonly GeneratedSection[] }) {
  const m = useMsg();
  const { matches } = useSettingsSearch();
  const visible = sections
    .map((section) => ({ section, entries: section.entries.filter((e) => e.featureExposed && matches(e)) }))
    .filter(({ entries }) => entries.length > 0);
  return (
    <div className="settings-sections" data-settings-area={area}>
      {visible.map(({ section, entries }) => (
        <section key={section.id} className="settings-section" data-section={section.id}>
          <h2 className="settings-section__title">{m(section.title)}</h2>
          {entries.map((entry) => (
            <SettingField key={entry.key} entry={entry} />
          ))}
        </section>
      ))}
    </div>
  );
}
