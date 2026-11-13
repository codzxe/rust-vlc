import type { ComponentType } from "react";
import { SETTINGS_ENTRIES } from "./generated/schema";
import type { SettingEntry } from "./generated/schema";
import { LocalePicker } from "./areas/language/LocalePicker";
import { ThemeModePicker } from "./areas/appearance/ThemeModePicker";
import { AccentColorPicker } from "./areas/appearance/AccentColorPicker";
import { InvertedSwitchField } from "./SettingWidgets";
import { SettingsSliderField } from "./SettingWidgets";
import { StartupVolumeField } from "./SettingWidgets";

// specialWidgets.ts — w26.k12 (j12 P2-5): the per-key special-widget registry
// as a LEAF module. It used to live in areas/index.tsx, which SettingField
// imported it from while areas/index imports SettingField right back for its
// composed screens — a production module cycle that only worked through ESM
// live-binding luck (the binding read at render time only) and was
// bundler/interop fragile. Both sides now import this leaf: SettingField for
// the per-row lookup, areas/index for the frozen registry re-export
// (SPECIAL_FIELD_WIDGETS keeps its public name/shape). The leaf imports only
// leaves — the pickers (areas/language · areas/appearance; neither imports
// SettingField/areas back), the SettingWidgets implementations, and the
// generated schema — so nothing below imports SettingField and the cycle is
// broken at the module level.
//
// Registry contract (unchanged, moved verbatim from areas/index): every key
// MUST be a real schemas/settings.toml key (AC-asserted in areas.test.tsx).
//
// w26.f03 (e05 F2/F4): the accent picker wires wherever the seed carries the
// accent key (`global.accent-color` per the finding; `interface.accent`/
// `global.accent` per the settingsThemeSync gap note), and the theme picker
// follows the theme key if the seed moves it from `interface.theme` to
// `global.theme`. Both registrations are DERIVED from the schema so the AC
// (every key real) holds at every merge state of the parallel f01 seed lane.
const THEME_SETTING_KEYS: readonly string[] = ["interface.theme", "global.theme"];
const ACCENT_SETTING_KEYS: readonly string[] = ["global.accent-color", "interface.accent", "global.accent"];

const themeEntry = SETTINGS_ENTRIES.find((e) => THEME_SETTING_KEYS.includes(e.key));
const accentEntry = SETTINGS_ENTRIES.find((e) => ACCENT_SETTING_KEYS.includes(e.key));

// w26.gfix-B: the frame's slider rows (g11-P2-3 / g12-P1-3), the inverted
// start-playing switch (g11-P2-2) and the Values-1 startup-volume control
// (g12-P1-4) — every key a real schema key (AC in areas.test.tsx).
const SLIDER_WIDGET_KEYS: readonly string[] = [
  "video.hue",
  "video.brightness",
  "video.contrast",
  "video.saturation",
  "video.gamma",
  "playback.default-rate",
  "audio.maximum-volume",
  "interface.hide-controls-after",
];

/**
 * Per-setting-key special widgets (card 4.E2 group 1). SettingField consults
 * this registry BEFORE its type-based default renderer — the framework seam.
 */
export const SPECIAL_FIELD_WIDGETS: Readonly<Record<string, ComponentType<{ entry: SettingEntry }>>> = {
  ...(themeEntry !== undefined ? { [themeEntry.key]: ThemeModePicker } : {}),
  ...(accentEntry !== undefined ? { [accentEntry.key]: AccentColorPicker } : {}),
  "global.language": LocalePicker,
  "playback.start-paused": InvertedSwitchField,
  "audio.startup-volume": StartupVolumeField,
  ...Object.fromEntries(SLIDER_WIDGET_KEYS.map((key) => [key, SettingsSliderField])),
};
