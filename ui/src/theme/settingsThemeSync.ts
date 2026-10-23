// Settings→theme consumption seam (card 4.E2; ThemeProvider itself is 4.A2's).
//
// E1 left the settings-schema ↔ ThemeProvider wiring open (ui/NEXT-STEPS.md
// "Settings wiring"): the `interface.theme` schema key must drive ThemeProvider
// state (and thereby the `data-theme` attribute on <html>). This hook is that
// seam — it lives on the theme side of the boundary and consumes the settings
// context, so ThemeProvider stays rendering-side only.
//
// The draft value is the source (live preview): editing the theme picker
// recolors immediately; Revert rolls the attribute back with the draft.
//
// GAP (schema owner, 1.B2 follow-up): there is NO `interface.accent` setting
// key in schemas/settings.toml, so no accent picker is rendered and the accent
// is never written from settings (config-never-invents, doc 05 §6.4). When the
// key lands, extend the sync below + a picker widget beside ThemeModePicker.

import { useContext, useEffect } from "react";
import { useSettings } from "../screens/settings/SettingsProvider";
import { SETTINGS_ENTRIES } from "../screens/settings/generated/schema";
import { ThemeContext } from "./ThemeProvider";
import type { ThemeModeSetting } from "./ThemeProvider";

/** The schema key this seam consumes (absent key ⇒ honest no-op). */
export const THEME_SETTING_KEY = "interface.theme";

const themeEntry = SETTINGS_ENTRIES.find((e) => e.key === THEME_SETTING_KEY);

const isThemeMode = (v: unknown): v is ThemeModeSetting => v === "dark" || v === "light";

/**
 * Push the settings draft theme into ThemeProvider (mode setting only —
 * "auto" is not in the schema enumeration yet; see the gap note above).
 * Mount once inside <SettingsProvider> (the settings shell does).
 */
export function useSettingsThemeSync(): void {
  const { value } = useSettings();
  // Optional consumer: without a ThemeProvider host there is nothing to sync
  // into — the settings screen stays renderable (and the hook stays inert).
  const theme = useContext(ThemeContext);
  const draft = themeEntry === undefined ? undefined : value(THEME_SETTING_KEY);

  useEffect(() => {
    if (theme === null) return;
    if (!isThemeMode(draft)) return; // unknown/absent value — never guess
    theme.setModeSetting(draft);
  }, [draft, theme]);
}
