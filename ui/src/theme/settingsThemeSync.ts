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
// PICKER already renders the third cell (Light · Dark · System, f03 lane)
// writing "auto"; the schema enumeration (schemas/settings.toml
// `interface.theme`) is still dark/light only — the REAL engine write path
// (config.set-setting validates against the embedded schema) will reject
// "auto" until the schema owner adds it. Note for the Lead (f01's file).
//
// w26.f08 ROUND 2 — the ACCENT half of the finding: the schema grew
// `global.accent-color` (f01 seed extension, 9-value enum, default "orange",
// same overrides store) and the f03 AccentColorPicker previews it live on
// /settings/global — but ThemeProvider still hardcoded `accent = "orange"`,
// so a persisted accent (violet, gray, …) was ignored on every fresh load
// outside /settings — exactly the theme bug's shape. This seam now covers it
// symmetrically: `readPersistedAccent()` bootstraps the provider's first
// render, and the sync below pushes the accent draft (live preview) wherever
// a SettingsProvider is mounted, so accent + mode travel together.

import { useContext, useEffect } from "react";
import { useSettings } from "../screens/settings/SettingsProvider";
import { localStorageSettingsPersistence } from "../screens/settings/persistence";
import { SETTINGS_ENTRIES } from "../screens/settings/generated/schema";
import { ACCENTS, type AccentId } from "../tokens/generated/tokens";
import { ThemeContext, type ThemeModeSetting } from "./context";

/** The schema keys this seam consumes (absent key ⇒ honest no-op). */
export const THEME_SETTING_KEY = "interface.theme";
export const ACCENT_SETTING_KEY = "global.accent-color";

const themeEntry = SETTINGS_ENTRIES.find((e) => e.key === THEME_SETTING_KEY);
const accentEntry = SETTINGS_ENTRIES.find((e) => e.key === ACCENT_SETTING_KEY);

/** Dark/light, or "auto" (follow the OS — ThemeProvider resolves it). */
const isThemeMode = (v: unknown): v is ThemeModeSetting =>
  v === "dark" || v === "light" || v === "auto";

/** One of the generated 9 accents (ThemeProvider.setAccent enforces these). */
const isAccentId = (v: unknown): v is AccentId =>
  typeof v === "string" && (ACCENTS as readonly string[]).includes(v);

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
 * Bootstrap read of the persisted `global.accent-color` override (w26.f08
 * round 2 — the accent half of e03-F1/e19-F1).
 *
 * Same seam and discipline as `readPersistedThemeMode`: total (absent/corrupt
 * store or absent schema key ⇒ null), never throws, never guesses. A value is
 * valid only when it is one of the GENERATED accents — the exact set
 * `ThemeProvider.setAccent` accepts (the schema enum and the generated ids
 * agree today; the runtime gate is the provider's, so it stays the truth).
 */
export function readPersistedAccent(): AccentId | null {
  if (accentEntry === undefined) return null; // no schema key — nothing to read
  const persisted = localStorageSettingsPersistence.load()[ACCENT_SETTING_KEY];
  return isAccentId(persisted) ? persisted : null;
}

/**
 * Push the settings draft theme AND accent into ThemeProvider ("auto"
 * included — it resolves against the OS preference inside the provider).
 * Mount once inside <SettingsProvider> (the settings shell does); the effect
 * is idempotent per provider pair (equal values are React no-ops), so the
 * pickers' own preview effects can stay mounted beside it.
 */
export function useSettingsThemeSync(): void {
  const { value } = useSettings();
  // Optional consumer: without a ThemeProvider host there is nothing to sync
  // into — the settings screen stays renderable (and the hook stays inert).
  const theme = useContext(ThemeContext);
  const draft = themeEntry === undefined ? undefined : value(THEME_SETTING_KEY);
  const accentDraft =
    accentEntry === undefined ? undefined : value(ACCENT_SETTING_KEY);

  useEffect(() => {
    if (theme === null) return;
    if (isThemeMode(draft)) {
      theme.setModeSetting(draft); // unknown/absent value — never guess
    }
    // setAccent THROWS outside the generated 9 — guard before every call.
    if (isAccentId(accentDraft)) {
      theme.setAccent(accentDraft);
    }
  }, [draft, accentDraft, theme]);
}
