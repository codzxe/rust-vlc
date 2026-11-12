// w26.f03 lane keys (settings shell + global/interface areas) — wave-26f
// fixer lane for the e05/e06 findings (F2 swatches · F4 segmented theme ·
// F5 language dropdown · F6 General retitle · F7 advanced-bar placement ·
// F9 search placeholder). Registered into ui/src/i18n/messages.ts by the
// Lead after the wave merge (contract §4.5); until then the settings
// screens resolve these through the lane-key seams (SettingsScreen.tsx
// `useSettingsMsg`, the appearance pickers), so the copy renders
// identically before and after registration. Frame provenance per key.
export const keys = {
  // RETITLE (existing msgid, Figma sidebar truth — the b02 `library`
  // precedent, "settings.area.library" → "Library"): the registered copy is
  // "Global"; the 13-area Figma sidebar labels the row "General" while the
  // area id (route, registry, window title "Global Settings") stays
  // `global` (tools-settings.md §D2 pitfall 3; Settings - global.svg
  // "setting category" row 1). The Lead retitles ui/src/i18n/messages.ts
  // at registration; the f03 seam prefers this value until then.
  "settings.area.global": "General",

  // Top-bar search pill placeholder (Settings - global.svg Frame 311:
  // "Search" #404040 ≈15px; tools-settings §C0). The registered
  // "settings.search.label" ("Search settings") stays the input's
  // accessible name — the frame copy is placeholder-only (e05 F9).
  "settings.search.placeholder": "Search",

  // Theme segmented control — third cell (Settings - global.svg "Theme"
  // Values_2: 320×36 track, 3×104px cells, active #371F06 + #D37918; cell
  // labels are outlined in the frame, [VERIFY] per tools-settings §C1).
  // "System" follows the onboarding appearance vocabulary
  // ("onboarding.appearance.theme.system"); the value is the ThemeProvider's
  // own mode-setting id ("auto", theme/context.ts) pending the schema enum
  // growth (w26.f01). The dark/light cells reuse the registered enum-value
  // msgids ("settings.value.interface-theme.dark"/-light).
  "settings.value.interface-theme.auto": "System",
} as const;
