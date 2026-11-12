// w26.gfixB lane keys (settings composition fixer — issue #62 wave g) — every
// msgid introduced by the frame re-seed of schemas/settings.toml and the
// composed settings areas (findings w26.g10–g14): the interface frame's
// Skin/Window/Playback Controls rows, the global Appearance/Startup moves
// (theme + "Check for updates automatically"), the playback Startup relabels
// ("Start playing as soon as a file opens" / "Default playback speed"), the
// audio startup-volume mode (a19 B5 Values-1), the hand action rows (Clear
// history, Reset picture), the controls casing fixes (g13-P2-9) and the
// staged Remotes rows (g13-P2-8).
//
// Registered into ui/src/i18n/messages.ts by the Lead after the wave merge
// (contract §4.5); until then the settings screens resolve these via the
// per-file lane seams (SettingField / SettingsSections / areas/*). All copy
// is frame-verbatim (tools-settings.md §C1–C8 + Values-1.svg).
export const keys = {
  // --- Section titles (new [[setting]] sections, w26.gfix-B seed) -----------
  "settings.section.interface.skin": "Skin",
  "settings.section.interface.playback-controls": "Playback Controls",
  "settings.section.controls.remotes": "Remotes",
  // The seed's home for the 3 non-frame playback rows (preferred audio/
  // subtitle languages + subtitle mode) — hidden from the playback screen's
  // own composition (PLAYBACK_HIDDEN_KEYS), search-reachable only. The frame
  // (C4) carries no such section; the title existed in no catalog (f01 gap).
  "settings.section.playback.track-languages": "Track languages",
  // Hand header-only section (frame C2 y837: "Notifications and System Tray",
  // content below the fold — no rows are exported).
  "settings.section.interface.notifications": "Notifications and System Tray",

  // --- Row labels — global (frame C1; w26.g10 P1-2 / P2-1) -------------------
  // (the theme row keeps its registered settings.interface.theme msgid —
  // only its PLACEMENT moved to global/appearance)
  "settings.global.interface-language": "Interface language",
  "settings.global.update-checks": "Check for updates automatically",

  // --- Row labels — interface (frame C2; w26.g10 P1-1) -----------------------
  "settings.interface.use-custom-skin": "Use a custom skin",
  "settings.interface.skin-file": "Skin file",
  "settings.interface.play-video-in-main-window": "Play video inside main window",
  "settings.interface.resize-to-fit-video": "Resize window to fit video",
  "settings.interface.remember-window-size-position": "Remember window size and position",
  "settings.interface.keep-above": "Keep window above other windows",
  "settings.interface.window-decorations": "Show window decorations",
  "settings.interface.filename-in-title-bar": "Show the file name in the title bar",
  "settings.interface.open-fullscreen-on": "Open fullscreen on",
  "settings.interface.start-in-compact-mode": "Start in compact mode",
  "settings.interface.auto-hide-controls": "Auto-hide the controls",
  "settings.interface.hide-controls-after": "Hide after",
  "settings.interface.show-controls-in-fullscreen": "Show controls in fullscreen",

  // --- Row labels — playback relabels (frame C4; w26.g11 P2-2) ---------------
  "settings.playback.start-playing": "Start playing as soon as a file opens",
  "settings.playback.default-speed": "Default playback speed",

  // --- Row labels — audio (frame C6 / Values-1.svg; w26.g12 P1-4) ------------
  "settings.audio.startup-volume-mode": "Startup volume mode",
  // The fallback Output-device trigger value (C6: dropdown "System default"
  // while the enumeration is absent — the honest engine fallback output).
  "settings.audio.device-system-default": "System default",

  // --- Hand action rows (no [[setting]] key — area composition) --------------
  // Path-row companion action (C0 control inventory: "Browse" 100×32 rx6).
  "settings.field.browse": "Browse",
  // Frame C3 y244: "Clear history now" → Clear button 100×32 (g11-P1-1).
  "settings.privacy.clear-history": "Clear history now",
  "settings.privacy.clear": "Clear",
  "settings.privacy.clear-unavailable": "History clearing arrives with the playback-history store",
  // Frame C5 y551: "Reset picture" → Reset button 100×32 (g12-P2-7).
  "settings.video.reset-picture": "Reset picture",
  "settings.video.reset": "Reset",

  // --- Controls area (frame C8; w26.g13 P2-8 / P2-9) -------------------------
  // Casing per frame + sibling-section consistency ("Skip Amounts"/"Mouse"
  // are title case; the b07 sentence-case keys render these until the Lead
  // registers the retitle).
  "settings.section.controls.shortcuts": "Keyboard Shortcuts",
  "settings.controls.edit-shortcuts-action": "Edit Keyboard Shortcuts",
  // Remotes (C8 y762): staged inert rows — the control server is Phase 5+
  // [PROPOSAL] (product.toml [features] control-server = false).
  "settings.controls.web-remote": "Web remote control",
  "settings.controls.port": "Port",
  "settings.controls.remotes-staged": "Staged — the web remote activates when the control server ships",

  // --- Enum option labels (generator-derived settings.value.<key>.<value>) --
  "settings.value.interface-resize-to-fit-video.never": "Never",
  "settings.value.interface-resize-to-fit-video.always": "Always",
  "settings.value.interface-open-fullscreen-on.current-screen": "Current screen",
  "settings.value.interface-open-fullscreen-on.all-screens": "All screens",
  "settings.value.interface-auto-hide-controls.fullscreen-only": "In fullscreen only",
  "settings.value.interface-auto-hide-controls.always": "Always",
  "settings.value.interface-auto-hide-controls.never": "Never",
  "settings.value.audio-startup-volume-mode.remember-last": "Remember last volume",
  "settings.value.audio-startup-volume-mode.always": "Always starts at:",
} as const;
