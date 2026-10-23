import type { ReactElement } from "react";
import { RadioGroup } from "../../../../components/form/RadioGroup";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import type { SettingEntry } from "../../generated/schema";
import { useSettings } from "../../SettingsProvider";
import { AVAILABLE_LOCALES, localeLabel } from "./locales";

// LocalePicker — card 4.E2 special widget for the `global.language` schema
// key (Language area). Options are the INTERSECTION of the schema enumeration
// and the localization crate's real available locales (locales/ dir) — real
// locales only, never invented. The value binds to the settings draft; the
// entry is schema-flagged restart_required, which the framework badges.

export function LocalePicker({ entry }: { entry: SettingEntry }): ReactElement {
  const m = useMsg();
  const { value, setValue } = useSettings();

  // Real locales only: a schema value without a shipped catalog is not
  // offered (and a shipped locale missing from the schema cannot be bound).
  const options = entry.enumeration
    .filter((o) => AVAILABLE_LOCALES.includes(o.value))
    .map((o) => ({ value: o.value, label: localeLabel(o.value, m) }));
  const current = String(value(entry.key));

  return (
    <div className="settings-field__special">
      <RadioGroup
        label={m(entry.description as MessageKey)}
        options={options}
        value={current}
        onChange={(next) => setValue(entry.key, next)}
      />
    </div>
  );
}
