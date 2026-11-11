// w26.c5 — tools integration lane. Genuinely new keys (CONTRACT §4.5): the
// copy for mounting the wave-c tools into the app — the Settings → Audio
// Effects section rows (Settings - audio-1.svg) and the player Tools dialog
// (ToolsDialogShell + Tabs). The Lead registers these into
// ui/src/i18n/messages.ts after the wave merge; until then effects.tsx +
// ToolsDialog.tsx resolve them seam-first (central catalog → this file) via
// their useEffectsMsg / useToolsMsg resolvers (the w26.c1 useEqMsg pattern),
// so the copy renders identically before and after registration.
//
// NOT duplicated here (reused through the same seams — grep first, §4.5):
//   * "Equalizer" + "Equalizer preset" — w26.c1's screen.eq.title /
//     screen.eq.preset (ui/src/i18n/keys/w26.c1.ts), resolved by including
//     that lane's keys in this lane's resolver catalog;
//   * the tools' own row/name copy — w26.c1/c2/c3/c4 lane keys, resolved by
//     the tools themselves;
//   * the staged-command badge copy — central settings.effects.
//     pending-command ("Engine stage ready — …") for the settings rows;
//     the tool-card staged notes live with their lanes.
export const keys = {
  // Settings → Audio, the Effects section header (frame: Settings -
  // audio-1.svg Frame 99 "Effects" — plain "Effects", not the old
  // "Audio effects"/"Video adjust" split keys).
  "settings.section.effects": "Effects",
  // Effects rows (frame: Settings - audio-1.svg, top → bottom after the EQ
  // panel). "Equalizer" itself reuses w26.c1's screen.eq.title.
  "settings.effects.two-pass": "Two-pass",
  "settings.effects.spatial-audio": "Spatial audio",
  "settings.effects.spatial-audio-mode": "Spatial audio mode",
  // The mode dropdown's frame-selected option (the frame shows no open list
  // — one honest frame-verbatim option, no invented engine modes).
  "settings.effects.spatial-mode.headphone-virtualization": "Headphone virtualization",
  "settings.effects.stereo-widening": "Stereo widening",
  // The EQ embed's staged note (w26.c1's tool ships no staged marker of its
  // own; this lane's wrapper carries the honest-unbound line, same copy
  // family as w26.c2's player.tools.dynamics.staged).
  "player.tools.equalizer.staged":
    "Staged — controls activate with the audio effects command card",
  // Player Tools dialog (ToolsDialogShell + Audio/Video tabs; no combined
  // frame exists — the tab labels name the tool-frame families
  // "Audio - *.svg" / "Video - *.svg").
  "player.tools.dialog.title": "Tools",
  "player.tools.tab.audio": "Audio",
  "player.tools.tab.video": "Video",
  // Accessible name of the (Lead-wired) player affordance that opens the
  // tools dialog — the gear/menu trigger ControlBar owns.
  "player.tools.open": "Audio and video tools",
} as const;
