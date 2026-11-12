// w26.f04 lane keys (Settings → Subtitles area) — the copy for the
// `Settings - subtitles.svg` frame (analysis tools-settings.md §C7): 3
// section headers + 14 row labels + the staged dropdown/segmented values.
// Registered into ui/src/i18n/messages.ts by the Lead after the wave merge
// (contract §4.5); until then subtitles.tsx resolves these seam-first
// (central catalog → this file) via its useSubtitlesMsg resolver (the
// w26.c1 useEqMsg pattern), so the copy renders identically before and
// after registration.
//
// NOT duplicated here (reused through the same seams — grepped first):
//   * "Title with extra characters" — the REGISTERED
//     playlist.subtitles.match.extra (identical copy, sub filename.svg);
//   * the area/window titles — settings.area.subtitles (w26.b02) /
//     settings.window-title.subtitles (w26.b01);
//   * the shared staged-note family — settings.subtitles.staged below is
//     new copy (the awaited seam here is the settings schema seed, not the
//     effects command card).
export const keys = {
  // --- Section headers (Settings - subtitles.svg, top → bottom) -----------
  "settings.section.subtitles.tracks-languages": "Tracks & Languages",
  "settings.section.subtitles.loading": "Loading Subtitles",
  "settings.section.subtitles.appearance": "Appearance",

  // --- Row labels (frame y-order within each section) ---------------------
  // Tracks & Languages: Enable subtitles (y135) · Preferred language (y179)
  // · Text encoding (y216).
  "settings.subtitles.enable": "Enable subtitles",
  "settings.subtitles.preferred-language": "Preferred language",
  "settings.subtitles.text-encoding": "Text encoding",
  // Loading Subtitles: Load subtitle files automatically (y328) · File name
  // matching (y372).
  "settings.subtitles.auto-load": "Load subtitle files automatically",
  "settings.subtitles.file-name-matching": "File name matching",
  // Appearance: Font (y478) · Size values (y518) · Font size (y573) · Bold
  // (y617) · Text color (y650) · Text opacity (y704) · Outline thickness
  // (y765) · Outline color (y809) · Shadow (y848).
  "settings.subtitles.font": "Font",
  "settings.subtitles.size-values": "Size values",
  "settings.subtitles.font-size": "Font size",
  "settings.subtitles.bold": "Bold",
  "settings.subtitles.text-color": "Text color",
  "settings.subtitles.text-opacity": "Text opacity",
  "settings.subtitles.outline-thickness": "Outline thickness",
  "settings.subtitles.outline-color": "Outline color",
  "settings.subtitles.shadow": "Shadow",

  // --- Staged control values (frame-verbatim; one honest option per
  //     dropdown — the frames export no open lists, so no invented modes) --
  "settings.subtitles.preferred-language.system": "System language",
  "settings.subtitles.encoding.automatic": "Automatic",
  "settings.subtitles.font.system-sans-serif": "System, sans-serif",
  "settings.subtitles.size-values.relative": "Relative",
  "settings.subtitles.size-values.pixels": "Pixels",
  "settings.subtitles.font-size.automatic": "Automatic",
  "settings.subtitles.outline-thickness.normal": "Normal",
  // The two color fields show their hex VALUES (frame: "#FFFFFF" / "#000000"
  // in the trigger text) — values, not theme colors; the swatch consumes
  // the same string (see subtitles.css).
  "settings.subtitles.text-color.value": "#FFFFFF",
  "settings.subtitles.outline-color.value": "#000000",

  // --- Honest staged note (one per section, the effects-area family) ------
  "settings.subtitles.staged":
    "Staged — controls activate when the subtitles settings seed lands",
} as const;
