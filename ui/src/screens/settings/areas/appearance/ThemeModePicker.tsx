import type { ReactElement } from "react";
import { RadioGroup } from "../../../../components/form/RadioGroup";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import type { SettingEntry } from "../../generated/schema";
import { useSettings } from "../../SettingsProvider";
import { useSettingsThemeSync } from "../../../../theme/settingsThemeSync";

// ThemeModePicker — card 4.E2 special widget for the `interface.theme` schema
// key (Appearance area). Options come from the schema enumeration; the value
// binds to the settings draft; the settings→ThemeProvider seam
// (ui/src/theme/settingsThemeSync) turns draft changes into the live
// `data-theme` attribute on <html> — the wiring E1 left open.

export function ThemeModePicker({ entry }: { entry: SettingEntry }): ReactElement {
  const m = useMsg();
  const { value, setValue } = useSettings();
  // Mounted here so the picker's screen always previews live (the shell also
  // mounts it; the hook is idempotent per provider pair).
  useSettingsThemeSync();

  return (
    <div className="settings-field__special">
      <RadioGroup
        label={m(entry.description as MessageKey)}
        options={entry.enumeration.map((o) => ({ value: o.value, label: m(o.msgid as MessageKey) }))}
        value={String(value(entry.key))}
        onChange={(next) => setValue(entry.key, next)}
      />
      <p className="settings-field__hint">{m("settings.appearance.theme-note")}</p>
    </div>
  );
}
