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
  // w26.k18 (j18 F4): player.controls.fullscreen/exit-fullscreen/cast pruned from this lane file — verified dead
  // (unreferenced by any source; grandfathered in ui/i18n-dead-keys.json)
  // so a future Lead prune of the catalog cannot resurrect them via re-merge.
  /** Player.svg Frame 10 — the next-track transport glyph (right of the pill). */
  "player.controls.next": "Next",
  /** aria group name for the output cluster (fullscreen/cast/gear/menu). */
  "player.controls.group-output": "Player controls",
} as const;
