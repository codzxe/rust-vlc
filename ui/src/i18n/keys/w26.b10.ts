// w26.b10 lane keys (playlist options layer) — copy with no existing catalog
// entry. Registered into ui/src/i18n/messages.ts by the Lead after the wave
// merge (contract §4.5); until then the playlist options screens resolve
// these via the local seam-first resolver (see
// ui/src/screens/playlist/options/useOptionsMsg.ts), so the copy renders
// identically before and after registration.
//
// Copy provenance: `Sorting Options.svg` / `View Options.svg` carry REAL
// outlined text (not placeholders). Labels are frame-verbatim (Title Case —
// the frames win over the general sentence-case rule, contract §3):
//   Sorting Options.png rows 1–16 + View Options.png rows 1–2.
export const keys = {
  // Trigger aria-labels (the sidebar meta-row sort / list-view icon buttons).
  "playlist.options.sort": "Sorting options",
  "playlist.options.view": "View options",
  // Sorting Options.svg — 16 rows, top to bottom. Row order is frame order.
  "playlist.options.sort.unwatched-first": "Unwatched First",
  "playlist.options.sort.watched-first": "Watched First",
  "playlist.options.sort.newest-first": "Newest First",
  "playlist.options.sort.oldest-first": "Oldest First",
  "playlist.options.sort.title-asc": "Title A-Z",
  "playlist.options.sort.title-desc": "Title Z-A",
  "playlist.options.sort.artist-asc": "Artist A-Z",
  "playlist.options.sort.artist-desc": "Artist Z-A",
  "playlist.options.sort.album-asc": "Album A-Z",
  "playlist.options.sort.album-desc": "Album Z-A",
  "playlist.options.sort.track-number-asc": "Track Number Ascending",
  "playlist.options.sort.track-number-desc": "Track Number Descending",
  "playlist.options.sort.uri-asc": "URI Ascending",
  "playlist.options.sort.uri-desc": "URI Descending",
  "playlist.options.sort.disc-number-asc": "Disc Number Ascending",
  "playlist.options.sort.disc-number-desc": "Disc Number Descending",
  // View Options.svg — 2 rows (grid icon row 1, list icon row 2).
  "playlist.options.view.grid": "Grid View",
  "playlist.options.view.list": "List View",
  // Media row hover affordance (media.svg Vector 36–38: the "•••" dots).
  "playlist.options.entry-menu": "More actions",
  // Grid/list empty surfaces (the panel's queue empty state is the panel's;
  // these name the two view regions themselves).
  "playlist.options.grid-label": "Playlist thumbnails",
  "playlist.options.list-label": "Playlist media",
} as const;
