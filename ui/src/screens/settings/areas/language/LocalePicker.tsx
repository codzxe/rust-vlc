import type { ReactElement } from "react";
import { Dropdown } from "../../../../components/menu/Dropdown";
import type { MessageKey } from "../../../../i18n/messages";
import { keys as fixerKeys } from "../../../../i18n/keys/w26.gfixB";
import type { SettingEntry } from "../../generated/schema";
import { useSettings } from "../../SettingsProvider";
import { makeAreaMsg } from "../shared";
import { AVAILABLE_LOCALES, localeLabel } from "./locales";

// LocalePicker — the "Interface language" row control (w26.f03, e05 F5).
// Figma Settings - global.svg "Interface language": a CLOSED dropdown
// trigger — 320×36 rx6, current locale + chevron-down (§C0 control
// inventory) — not a stacked radio list. The compact Dropdown component
// (Setting Preset.svg grammar) carries the anatomy: combobox trigger,
// listbox popup with roving tabindex, arrows/Home/End, typeahead,
// Escape/Tab close with focus restoration.
//
// Options stay the INTERSECTION of the schema enumeration and the
// localization crate's real available locales (locales/ dir) — real
// locales only, never invented; the value binds to the settings draft
// (restart badge via the framework, the entry is schema-flagged
// restart_required).

/** usePickerMsg — useMsg + this lane's pre-registration keys (w26.k13: the
 * shared makeAreaMsg factory — the j13 ruling 1 dedup). */
const usePickerMsg = makeAreaMsg(fixerKeys);

export function LocalePicker({ entry }: { entry: SettingEntry }): ReactElement {
  const m = usePickerMsg();
  const { value, setValue } = useSettings();

  // Real locales only: a schema value without a shipped catalog is not
  // offered (and a shipped locale missing from the schema cannot be bound).
  const options = entry.enumeration
    .filter((o) => AVAILABLE_LOCALES.includes(o.value))
    .map((o) => ({ value: o.value, labelKey: o.value }));
  const current = String(value(entry.key));

  return (
    <div className="settings-field__special">
      <Dropdown
        label={m(entry.description as MessageKey)}
        options={options}
        resolveLabel={(tag) => localeLabel(tag, m)}
        value={current}
        onChange={(next) => setValue(entry.key, next)}
        density="compact"
        className="settings-field__dropdown"
      />
    </div>
  );
}
