// w26.i02 lane keys (player dark core/states — wave I fixers #2, h03).
// Registered into ui/src/i18n/messages.ts by the Lead after the wave merge;
// until then PlayerScreen resolves them through its lane seam-first resolver
// (contract §4.5), so the copy renders identically before and after.
//
// Reused registered keys (no duplicates here): "shell.sidebar.title"
// ("Playlists" — the frame's sidebar header, Player.svg), "shell.sidebar.
// create" ("Create playlist" — the frame's create row), "shell.sidebar.
// list-label" ("Playlists"), "shell.sidebar.empty" ("No playlists yet.").
export const keys = {
  // Player.svg sidebar meta row prints the count lowercase ("2 items" —
  // path-id "2 items" in Player.svg); the shell's a10 count suffix is
  // capitalized ("Items"), so the dock carries its own frame-exact suffix.
  "player.dock.count-suffix": "items",
  // The canvas context menu's Playlist submenu rows: the docked sidebar is
  // the playlists BROWSER since w26.i02 (h03 #1 — Player.png's sidebar), so
  // the toggle rows say playlists, not queue (the queue panel stays
  // reachable on /playlist and in music mode).
  "player.canvas.menu.show-playlists": "Show playlists panel",
  "player.canvas.menu.hide-playlists": "Hide playlists panel",
  // The dock header's own collapse chevron — Player.svg draws it active
  // (it hides the docked panel). Distinct from the menu row / music-mode
  // stage toggle copy above so every affordance has exactly one name.
  "player.dock.hide": "Hide the playlists dock",
  // The dock header's forward chevron — Player.svg draws it at opacity 0.3
  // (disabled); no honest forward surface exists in the dock, so it renders
  // disabled with this hint (the shell sidebar's more.hint pattern).
  "player.dock.forward": "Forward",
  "player.dock.forward.hint": "Forward navigation is not exposed by this build yet.",
  // w26.i02 pass 4 (Player-4/Player-3, the Lead's composition ruling): the
  // canvas's playlist-browse face + the floating now-playing card.
  /** Exit the browse face — back to the player canvas (header chevron). */
  "player.browse.exit": "Back to the player",
  /** The browse grid's accessible name (the playlist's media items). */
  "player.browse.label": "Playlist items",
  /** Honest empty face — the saved playlist records no item names. */
  "player.browse.empty": "No items in this playlist yet.",
  /** The floating now-playing card's region label (Player-3's window). */
  "player.preview.label": "Now playing",
  /** The browse header's PiP toggle — re-opens the collapsed card. */
  "player.preview.show": "Show the now-playing card",
  /** The toggle's disabled reason while no session is up (never a dead
   *  button — the hint carries the honesty, doc 05 §1 C4). */
  "player.preview.show.hint": "Open media to see the now-playing card.",
  /** The card's ✕ — collapse the floating window (browse continues). */
  "player.preview.hide": "Hide the now-playing card",
  /** The card's title fallback while no title is reported (honest empty). */
  "player.preview.title-empty": "No media open",
} as const;
