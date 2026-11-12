// Settings→theme consumption seam (card 4.E2; ThemeProvider itself is 4.A2's).
//
// E1 left the settings-schema ↔ ThemeProvider wiring open (ui/NEXT-STEPS.md
// "Settings wiring"): the `interface.theme` schema key must drive ThemeProvider
// state (and thereby the `data-theme` attribute on <html>). This hook is that
// seam — it lives on the theme side of the boundary and consumes the settings
// context, so ThemeProvider stays rendering-side only.
//
// w26.f08 (e03-F1 / e19-F1): the sync used to mount ONLY inside the settings
// screen + ThemeModePicker, so a persisted `interface.theme: "light"` was
// ignored on every fresh load of every non-settings route. The fix has two
// halves, both owned by this seam:
//   1. `readPersistedThemeMode()` — a synchronous bootstrap read of the
//      persisted overrides (same persistence seam + storage key as
//      SettingsProvider) that ThemeProvider's initial state lazy-initializes
//      from, so the FIRST render of EVERY route (player, playlist, discovery,
//      onboarding, …) starts on the persisted mode — no dark→light flash.
//   2. `useSettingsThemeSync()` — unchanged role: pushes the settings DRAFT
//      (live preview) into ThemeProvider wherever a SettingsProvider is
//      mounted (the settings screen still mounts it).
//
// w26.f08 (e19-F5): "auto" ("System") is now a first-class value on this seam
// — the sync forwards it and the bootstrap read accepts it, so a persisted
// "auto" hydrates and follows prefers-color-scheme end-to-end. The settings
// PICKER cannot offer it yet: the schema enumeration (schemas/settings.toml
// `interface.theme`) is dark/light only — surfacing the third radio is the
// picker lane's (w26.f03) + schema owner's move; the theme side is ready.
//
// GAP (schema owner, 1.B2 follow-up): there is NO `interface.accent` setting
// key in schemas/settings.toml, so no accent picker is rendered and the accent
// is never written from settings (config-never-invents, doc 05 §6.4). When the
// key lands, extend the sync below + a picker widget beside ThemeModePicker.

import { useContext, useEffect } from "react";
import { useSettings } from "../screens/settings/SettingsProvider";
import { localStorageSettingsPersistence } from "../screens/settings/persistence";
import { SETTINGS_ENTRIES } from "../screens/settings/generated/schema";
import { ThemeContext, type ThemeModeSetting } from "./context";

/** The schema key this seam consumes (absent key ⇒ honest no-op). */
export const THEME_SETTING_KEY = "interface.theme";

const themeEntry = SETTINGS_ENTRIES.find((e) => e.key === THEME_SETTING_KEY);

/** Dark/light, or "auto" (follow the OS — ThemeProvider resolves it). */
const isThemeMode = (v: unknown): v is ThemeModeSetting =>
  v === "dark" || v === "light" || v === "auto";

/**
 * Bootstrap read of the persisted `interface.theme` override (w26.f08).
 *
 * Reads the SAME persistence seam (storage key `lumen.settings.overrides.v1`,
 * corrupt-store tolerant) SettingsProvider hydrates from, so the two never
 * disagree about where the truth lives. Returns null for absent/unknown keys
 * and values that are not a theme mode — callers keep their default and never
 * guess (the same discipline as the sync hook below).
 */
export function readPersistedThemeMode(): ThemeModeSetting | null {
  if (themeEntry === undefined) return null; // no schema key — nothing to read
  const persisted = localStorageSettingsPersistence.load()[THEME_SETTING_KEY];
  return isThemeMode(persisted) ? persisted : null;
}

/**
 * Push the settings draft theme into ThemeProvider ("auto" included — it
 * resolves against the OS preference inside the provider). Mount once inside
 * <SettingsProvider> (the settings shell does).
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
