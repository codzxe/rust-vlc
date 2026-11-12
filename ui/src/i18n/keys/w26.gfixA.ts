// w26.gfixA lane keys (issue #62 fix cycle — customize modal rebuild).
//
// The frame-measured tile catalog of "Player - customize.svg" (31 items,
// rows 1–4) plus the toolbar-band edit overlay's a11y copy. Registered
// into ui/src/i18n/messages.ts by the Lead after the wave merge (contract
// §4.5); until then the customize renderer resolves these via its
// seam-first local resolver, so the copy renders identically before and
// after.
//
// Label style: sentence case per contract §2 (the frames print Title Case
// "Right Sidebar" / "Go Home" — the app-wide sentence-case rule wins, the
// same call w26.a17 made for "Forward 10s").
export const keys = {
  // Row 1 tiles (frame order) — engine-bound quick actions keep their
  // catalog keys (customize.tile.forward-10 …, player.controls.stop,
  // customize.action.repeat); these three are new.
  "customize.tile.spacer": "Spacer",
  "customize.tile.divider": "Divider",
  "customize.tile.playback-speed": "Playback speed",
  // Row 2 tiles.
  "customize.tile.record": "Record",
  "customize.tile.snapshot": "Snapshot",
  "customize.tile.pip": "Picture in picture",
  "customize.tile.hdr": "HDR",
  "customize.tile.zoom": "Zoom",
  "customize.tile.right-sidebar": "Right sidebar",
  "customize.tile.quit": "Quit VLC",
  "customize.tile.vlc-settings": "VLC settings",
  // Row 3 tiles.
  "customize.tile.go-home": "Go home",
  "customize.tile.thumbnail": "Thumbnail",
  "customize.tile.file-name": "File name",
  "customize.tile.artist-name": "Artist name",
  "customize.tile.folder-name": "Folder name",
  "customize.tile.playlist-name": "Playlist name",
  "customize.tile.properties": "Properties",
  "customize.tile.replay": "Replay",
  "customize.tile.chapters": "Chapters",
  // Row 4 tiles.
  "customize.tile.favorite": "Favorite",
  "customize.tile.bookmark": "Bookmark",
  "customize.tile.bookmarks": "Bookmarks",
  "customize.tile.playlists": "Playlists",
  // Toolbar-band edit overlay (frames -1..-7: the live control band turns
  // into dashed edit wells while the customize modal is open).
  "customize.band.label": "Toolbar preview",
  "customize.band.drop-hint": "Drop items here to add them to the toolbar",
  "customize.band.remove": "Remove {item} from the toolbar",
  "customize.tile.remove-hint": "Remove from toolbar",
} as const;
