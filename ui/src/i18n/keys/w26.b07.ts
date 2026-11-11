// w26.b07 lane keys (settings areas: controls + recording) — the Controls
// area's hand-authored copy (the Keyboard Shortcuts section + its two
// full-width actions; Settings - controls.svg §C8). Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge (contract §4.5,
// wave-d lane b19); until then controls.tsx resolves these via its lane-key
// seam (`useAreaMsg`, the SettingsSections pattern) so the copy renders
// identically before and after registration. All other Controls/Recording
// copy reuses existing keys: the seeded row/section labels live in
// ui/src/i18n/keys/w26.b02.ts (rendered by the generated screens' own seam)
// and the section title "Keyboard shortcuts" equivalents already exist under
// settings.hotkeys.* — the keys below are only what no catalog entry carries.
// Copy is sentence case per CONTRACT §2 (the frames' title-case export
// styling is normalized; frame provenance noted per key).
export const keys = {
  // Keyboard Shortcuts section (Settings - controls.svg §C8 y83: header +
  // "Defaults" and "Reset all shortcuts" rows with full-width 320×32 r6
  // action buttons, centered #8F8F8F labels). The hotkey bindings themselves
  // stay dynamic keymap data (areas/hotkeys), not [[setting]] rows — b02
  // seeded no section title for them, so the hand section title lives here.
  "settings.section.controls.keyboard-shortcuts": "Keyboard shortcuts",
  "settings.controls.defaults": "Defaults",
  "settings.controls.reset-all-shortcuts": "Reset all shortcuts",
  // Full-width action labels (frame: "Edit Keyboard Shortcuts" / "Reset").
  "settings.controls.edit-shortcuts": "Edit keyboard shortcuts",
  "settings.controls.reset-shortcuts": "Reset",
} as const;
