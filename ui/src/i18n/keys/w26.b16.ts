// w26.b16 lane keys (keyboard-shortcuts screen) — the Keyboard Shortcuts.svg
// frame copy that has no catalog entry yet. Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge (contract §4.5);
// until then the screen resolves these via its local seam-first resolver
// (see ShortcutsScreen.tsx `useShortcutsMsg`), so the copy renders
// identically before and after registration. All other shortcuts copy
// reuses the existing `shortcuts.*` / `settings.hotkeys.column.*` /
// `open.cancel` catalog keys (grepped first).
export const keys = {
  // Column headers (Keyboard Shortcuts.svg Frame 503: "Action" #6B6B6B
  // left, "Hotkey" / "Global Hotkey" centered over the two keycap columns).
  // "Action" reuses settings.hotkeys.column.action.
  "shortcuts.column.hotkey": "Hotkey",
  "shortcuts.column.global": "Global Hotkey",
  // Search-scope dropdown (Frame 124: 200×34 r6 #151515 trigger, value
  // "All Fields" #969696, chevron #8F8F8F). The label is the trigger's
  // accessible name (no visible label in the frame).
  "shortcuts.filter.label": "Search scope",
  "shortcuts.filter.all": "All Fields",
  // Footer confirm (Frame 141: 100×32 r6 #371F06, label "Save" #D37918,
  // 50% opacity — the read-only reference has nothing to save).
  "shortcuts.save": "Save",
  // Top-bar ✕ (Frame 207 Icon Wrapper: 10×10 #8F8F8F stroke glyph; returns
  // to the player).
  "shortcuts.close": "Close shortcuts",
} as const;
