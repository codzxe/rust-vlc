import { useContext, useEffect } from "react";
import type { ReactElement } from "react";
import { SegmentedControl } from "../../../../components/form/SegmentedControl";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import { keys as laneKeys } from "../../../../i18n/keys/w26.f03";
import type { SettingEntry } from "../../generated/schema";
import { useSettings } from "../../SettingsProvider";
import { ThemeContext } from "../../../../theme/context";
import type { ThemeModeSetting } from "../../../../theme/context";
import { useSettingsThemeSync } from "../../../../theme/settingsThemeSync";

// ThemeModePicker — the "Theme" row control (w26.f03, e05 F4). Figma
// Settings - global.svg "Theme" (Values_2): a segmented control — 320×36
// rx6 track, 3×104px cells, the active cell #371F06 + #D37918 — NOT a
// stacked radio list. Cells: Light · Dark · System (frame cell labels are
// outlined → [VERIFY] per tools-settings §C1; "System" follows the
// onboarding appearance-step vocabulary).
//
// Values: the schema enumeration is the truth for the cells it carries
// (dark/light today); the System cell writes the ThemeProvider's own
// "auto" mode-setting id (theme/context.ts) until the seed lane grows the
// enum — the value is still a real theme-system setting, never invented
// UI state. The draft binds through the settings provider (dirty/apply
// contract), and the ThemeContext seam previews every draft value live
// (the shell's useSettingsThemeSync only carries dark/light; this picker
// extends the preview to "auto" locally).

/** Pre-registration seam: this lane's keys (see w26.f03.ts). */
const laneCatalog: Readonly<Record<string, string>> = { ...laneKeys };

/** Label fallbacks per theme value (schema msgid first, vocabulary second). */
const LABEL_FALLBACK_KEYS: Readonly<Record<string, MessageKey>> = {
  light: "onboarding.appearance.theme.light",
  dark: "onboarding.appearance.theme.dark",
  system: "onboarding.appearance.theme.system",
  auto: "onboarding.appearance.theme.system",
};

/** Values the ThemeProvider accepts as a mode setting (theme/context.ts). */
const isModeSetting = (v: string): v is ThemeModeSetting =>
  v === "dark" || v === "light" || v === "auto";

export function ThemeModePicker({ entry }: { entry: SettingEntry }): ReactElement {
  const m = useMsg();
  const { value, setValue } = useSettings();
  // Mounted here so the picker's screen always previews live (the shell also
  // mounts it; the hook is idempotent per provider pair).
  useSettingsThemeSync();
  const theme = useContext(ThemeContext);
  const current = String(value(entry.key));

  // Frame cells in order. The System cell's value follows the schema when
  // the enumeration carries one ("auto"/"system"), else "auto".
  const systemValue = entry.enumeration.some((o) => o.value === "system") ? "system" : "auto";
  const cells = ["light", "dark", systemValue].map((setting) => {
    const option = entry.enumeration.find((o) => o.value === setting);
    const fallback = LABEL_FALLBACK_KEYS[setting];
    const label =
      option !== undefined
        ? (m(option.msgid as MessageKey) ?? laneCatalog[option.msgid] ?? option.value)
        : fallback !== undefined
          ? m(fallback)
          : setting;
    return { value: setting, label };
  });

  // Draft values outside the three cells (a stale/unknown persisted value)
  // render no active cell rather than guessing — matches the radiogroup
  // contract (an unknown value selects nothing).
  const display = cells.some((c) => c.value === current) ? current : "";
  const preview = current === "system" ? "auto" : current;

  // Live preview seam (card 4.E2): draft → ThemeProvider → data-theme on
  // <html>. Covers "auto", which the shell-level sync does not carry yet.
  useEffect(() => {
    if (theme === null || !isModeSetting(preview)) return;
    theme.setModeSetting(preview);
  }, [preview, theme]);

  return (
    <div className="settings-field__special">
      <SegmentedControl
        label={m(entry.description as MessageKey)}
        options={cells}
        value={display}
        onChange={(next) => setValue(entry.key, next)}
      />
    </div>
  );
}
