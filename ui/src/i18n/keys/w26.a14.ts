// w26.a14 — player control bar lane. Genuinely new keys introduced this lane
// (CONTRACT §4.5): the rebuilt bottom bar's additive controls (Player.svg
// bottom-bar composition). The Lead registers these into
// ui/src/i18n/messages.ts after the wave merge; the components meanwhile
// read their local CONTROLS_COPY module copy in
// ui/src/screens/player/controls/ControlBar.tsx (the established dialogs
// COPY-NOTE pattern, w26.a18). Reused keys NOT duplicated here:
// player.controls.stop/play/pause/restart/buffering-hint/seekbar/volume/
// mute/unmute/duration-unknown/group-*/reason.*, player.track.*,
// player.menu.app, player.empty.open, menu.file.open-network-stream,
// menu.help.keyboard-shortcuts, menu.view.settings.
export const keys = {
  /** Player.svg Frame 10 — the next-track transport glyph (right of the pill). */
  "player.controls.next": "Next",
  /** Fullscreen toggle, entering (exits reuse the same control). */
  "player.controls.fullscreen": "Enter fullscreen",
  /** Fullscreen toggle, exiting. */
  "player.controls.exit-fullscreen": "Exit fullscreen",
  /** Remote-playback control; honestly not-available until a surface exists. */
  "player.controls.cast": "Cast",
  /** aria group name for the output cluster (fullscreen/cast/gear/menu). */
  "player.controls.group-output": "Player controls",
} as const;
