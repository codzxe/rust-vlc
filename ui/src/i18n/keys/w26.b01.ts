// w26.b01 lane keys (settings shell) — the W26 Figma settings-window copy
// that has no catalog entry yet. Registered into ui/src/i18n/messages.ts by
// the Lead after the wave merge (contract §4.5); until then the settings
// shell resolves these via its local seam-first resolver (see
// SettingsScreen.tsx `useSettingsMsg`), so the copy renders identically
// before and after registration. Frame provenance per key.
export const keys = {
  // Sidebar group labels (Settings - global.svg Frame 260/261/263: "App",
  // "Media", "System" — the one place the design uses group labels).
  "settings.group.app": "App",
  "settings.group.media": "Media",
  "settings.group.system": "System",
  // In-flow advanced disclosure (Settings - global.svg Frame 265 — label +
  // ›/∨ chevron; global-1.svg is the expanded state).
  "settings.advanced-options": "Advanced Options",
  // Top-bar ✕ (Settings - global.svg Icon Wrapper: 10×10 #8F8F8F stroke
  // close glyph, 25px right inset; returns to the player).
  "settings.close": "Close settings",
  // Window titles (verbatim from the frames' top bars).
  "settings.window-title.global": "Global Settings",
  "settings.window-title.interface": "Interface Settings",
  "settings.window-title.privacy": "Privacy Settings",
  "settings.window-title.controls": "Mouse, Keyboard & Remote Controls",
  "settings.window-title.playback": "Playback Preferences",
  "settings.window-title.video": "Video Defaults",
  "settings.window-title.audio": "Audio Defaults",
  "settings.window-title.subtitles": "Subtitle Defaults",
  "settings.window-title.library": "Playlists, Folders & Files",
  "settings.window-title.network": "Network Connections & Security",
  "settings.window-title.performance": "Performance Settings",
  "settings.window-title.recording": "Recording, Casting & Streaming Out",
  "settings.window-title.extensions": "Add-ons & Scripts",
} as const;
