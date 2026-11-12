// w26.f06 lane keys (playlist surfaces shipping) — the copy for the newly
// mounted /playlist surfaces that has no existing catalog entry. Registered
// into ui/src/i18n/messages.ts by the Lead after the wave merge (contract
// §4.5); until then the playlist screen resolves these through the local
// seam-first resolver (see ui/src/screens/playlist/usePlaylistMsg.ts), so the
// copy renders identically before and after registration. Existing keys were
// grepped first — the browse count reuses `playlist.library.items-suffix`
// and the queue-row entry reuses `playlist.queue.menu.add-subtitles`.
export const keys = {
  // Back affordance: the opened-playlist browse view returns to the library.
  "playlist.browse.back": "All playlists",
  // Opened-playlist empty state (names-only items; nothing recorded yet).
  "playlist.browse.empty-title": "Nothing in this playlist yet",
  "playlist.browse.empty-body": "Add files while creating a playlist to fill it.",
  // Library row context menu: open the playlist's media browse view.
  "playlist.library.menu.open": "Open",
  // Toolbar action opening the Add-Subtitles flow (Add Subtitles frames).
  "playlist.subtitles.open": "Add subtitles",
} as const;
