// w26.c6 lane keys (context menus in screens) — copy for the Figma context
// menus wired into the real screens (contract §4.5): the queue-row menu
// (playlist context menu.svg), the volume-row menu (folder context menu.svg)
// and the player canvas menu (right-click menu.svg). The frames carry
// placeholder "Option" labels, so semantics derive from each row's pinned
// icon glyph; the labels below name those icons' actions in the app's
// sentence-case voice. Registered into ui/src/i18n/messages.ts by the Lead
// after the wave merge; until then the screens resolve them through their
// lane seam-first resolvers (see PlaylistPanel.tsx / DiscoveryScreen.tsx /
// PlayerScreen.tsx), so the copy renders identically before and after.
export const keys = {
  // Queue-row context menu (playlist context menu.svg — 8 icon rows).
  "playlist.queue.menu.title": "Queue item actions",
  "playlist.queue.menu.play": "Play",
  "playlist.queue.menu.favorite": "Favorite",
  "playlist.queue.menu.save": "Save playlist",
  "playlist.queue.menu.add-media": "Add media…",
  "playlist.queue.menu.add-subtitles": "Add subtitle tracks",
  "playlist.queue.menu.add-audio": "Add audio tracks",
  // Volume-row context menu (folder context menu.svg — 6 icon rows).
  "discovery.menu.title": "Volume actions",
  "discovery.menu.add-file": "Add file…",
  "discovery.menu.add-folder": "Add folder…",
  "discovery.menu.add-subtitles": "Add subtitle tracks",
  "discovery.menu.add-audio": "Add audio tracks",
  "discovery.menu.refresh": "Refresh",
  "discovery.menu.info": "Info",
  // Player canvas context menu (right-click menu.svg — REAL frame labels;
  // only the ones with no catalog entry are here, the rest reuse keys).
  "player.canvas.menu.title": "Player actions",
  "player.canvas.menu.previous": "Previous Item",
  "player.canvas.menu.next": "Next Item",
  "player.canvas.menu.hide-ui": "Hide UI",
  "player.canvas.menu.video": "Video",
  "player.canvas.menu.audio": "Audio",
  "player.canvas.menu.subtitle": "Subtitle",
  "player.canvas.menu.more-settings": "More Settings",
  "player.canvas.menu.open-media": "Open Media",
  "player.canvas.menu.quit": "Quit VLC",
  "player.canvas.menu.show-queue": "Show queue panel",
  "player.canvas.menu.hide-queue": "Hide queue panel",
  "player.canvas.menu.open-playlist": "Open playlist",
} as const;
