// w26.gfixC lane keys — the control-bar/right-cluster + player-home labels
// (CONTRACT §4.5: catalog first, lane copy while the Lead's post-wave merge
// is pending — the ControlBar/StageOverlays seam-first resolvers render
// identical copy before and after).
//
// Reused catalog keys for the sibling surfaces: menu.playback.previous
// ("Previous" — the left transport slot), player.controls.next ("Next"),
// player.controls.group-tracks, player.controls.reason.no-media / .unknown,
// player.track.auto / .disable / .unknown, menu.playback.shuffle,
// playlist.controls.repeat.off|all|one, player.canvas.menu.show-queue /
// .hide-queue (the docked queue panel seam labels),
// playlist.entry.unknown-title.
export const keys = {
  // w26.k18 (j18 F4): player.controls.settings-popup pruned from this lane file — verified dead
  // (unreferenced by any source; grandfathered in ui/i18n-dead-keys.json)
  // so a future Lead prune of the catalog cannot resurrect them via re-merge.
  /** Current_Time.svg — the position timecode button's accessible name. */
  "player.controls.time-current": "Current time",
  /** Duration.svg — the total-duration timecode button's accessible name. */
  "player.controls.time-duration": "Total duration",
  /** Duration.svg state=click — the "-00:23:30" remaining toggle face. */
  "player.controls.time-remaining": "Remaining time",
  /** Controls.svg popovers' 360×32 r4 footer (Button.svg default grammar). */
  "player.controls.advanced-settings": "Show Advanced Settings",
  /** The audio trigger's popover (Controls.svg right cluster, 5-bar glyph). */
  "player.controls.audio-tracks": "Audio tracks",
  /** The CC trigger's popover (Controls.svg right cluster). */
  "player.controls.subtitle-tracks": "Subtitle tracks",
  /** The queue trigger's popover (Controls.svg "Queue" tab). */
  "player.controls.queue": "Queue",
  /** Queue popover honest empty face — never a fabricated list. */
  "player.controls.queue-empty": "Nothing in the queue",
  /** Player-5.svg section 1 header (y≈157). */
  "player.home.continue-watching": "Continue Watching",
  /** Player-5.svg section 2 header (y≈505). */
  "player.home.listen-again": "Listen Again",
  /** Player-5.svg section 3 header (y≈894). */
  "player.home.folders": "Frequently Visited Folders",
  /** Home sections with no engine seam yet — honest, discoverable, inert. */
  "player.home.section-empty": "Nothing here yet",
  /** Home card action — re-opens the remembered source (real affordance). */
  "player.home.play": "Play",
} as const;
