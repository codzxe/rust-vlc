// w26.b02 lane keys (settings area registry + seed) — every msgid introduced
// by the 13-area reshape and the schemas/settings.toml seed (controls /
// recording / extensions [[setting]] rows + the hand-registered video / audio
// / subtitles area titles). Registered into ui/src/i18n/messages.ts by the
// Lead after the wave merge (contract §4.5; wave-d lane b19 carries the
// registration) — until then the settings registry resolves these via its
// lane-key seam (areas/index.tsx), and the generated screens render them
// once the catalog carries the keys. All other settings copy reuses the
// existing `settings.*` catalog keys (grepped first).
export const keys = {
  // --- Area titles (Figma Settings sidebar, analysis D2/D3) ---------------
  // Sidebar labels are the SHORT forms; the long window titles ("Mouse,
  // Keyboard & Remote Controls", …) are shell-chrome copy (b01 lane).
  "settings.area.controls": "Controls",
  "settings.area.video": "Video",
  "settings.area.audio": "Audio",
  "settings.area.subtitles": "Subtitles",
  "settings.area.recording": "Recording",
  "settings.area.extensions": "Extensions",
  // RETITLE (existing msgid, Figma sidebar truth): the registered copy is
  // "Playlist & Services"; the 13-area Figma sidebar labels the row
  // "Library" (Settings - library.svg / Settings.png sidebar). The Lead
  // retitles ui/src/i18n/messages.ts at registration — carrying it here
  // makes the lane-catalog seam render the correct label pre-registration.
  "settings.area.library": "Library",

  // --- Section titles (schemas/settings.toml [areas] seed, w26.b02) --------
  "settings.section.controls.skip-amounts": "Skip Amounts",
  "settings.section.controls.mouse": "Mouse",
  "settings.section.recording.recording": "Recording",
  "settings.section.extensions.add-ons": "Add-ons",

  // --- Row labels (Figma frames C8/C12/C13) --------------------------------
  "settings.controls.skip-very-short": "Very short skip",
  "settings.controls.skip-short": "Short skip",
  "settings.controls.skip-medium": "Medium skip",
  "settings.controls.skip-long": "Long skip",
  "settings.controls.mouse-single-click": "Single click on the video",
  "settings.controls.mouse-double-click": "Double click on the video",
  "settings.controls.mouse-scroll-wheel": "Scroll wheel",
  "settings.controls.mouse-horizontal-scroll": "Horizontal scroll",
  "settings.controls.mouse-reverse-scroll-direction": "Reverse scroll direction",
  "settings.recording.directory": "Save recordings to",
  "settings.recording.format": "Recording format",
  "settings.extensions.load-on-startup": "Load add-ons on startup",

  // --- Enum option labels (generator-derived
  //     settings.value.<key>.<value> msgids) --------------------------------
  "settings.value.controls-mouse-single-click.play-pause": "Play / Pause",
  "settings.value.controls-mouse-single-click.toggle-fullscreen": "Toggle fullscreen",
  "settings.value.controls-mouse-single-click.show-menu": "Show menu",
  "settings.value.controls-mouse-single-click.none": "Do nothing",
  "settings.value.controls-mouse-double-click.toggle-fullscreen": "Toggle fullscreen",
  "settings.value.controls-mouse-double-click.play-pause": "Play / Pause",
  "settings.value.controls-mouse-double-click.show-menu": "Show menu",
  "settings.value.controls-mouse-double-click.none": "Do nothing",
  "settings.value.controls-mouse-scroll-wheel.volume": "Volume",
  "settings.value.controls-mouse-scroll-wheel.seek": "Seek",
  "settings.value.controls-mouse-scroll-wheel.none": "Do nothing",
  "settings.value.controls-mouse-horizontal-scroll.seek": "Seek",
  "settings.value.controls-mouse-horizontal-scroll.volume": "Volume",
  "settings.value.controls-mouse-horizontal-scroll.none": "Do nothing",
  "settings.value.recording-format.mp4-remux": "MP4 (stream copy)",
  "settings.value.recording-format.wav": "WAV",
} as const;
