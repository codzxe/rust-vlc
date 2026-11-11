// w26.b09 lane keys (playlist core) — the /playlist library screen + the
// create-playlist dialog (new playlist.svg) copy that has no existing catalog
// entry. Registered into ui/src/i18n/messages.ts by the Lead after the wave
// merge (contract §4.5); until then this screen resolves its keys via the
// lane seam-first resolver (see PlaylistScreen.tsx `usePlaylistMsg`), so the
// copy renders identically before and after registration.
export const keys = {
  // Playlists library panel (playlist.svg — "Cool Playlist" rows).
  "playlist.library.count-suffix": "playlists",
  "playlist.library.list-label": "Playlists",
  "playlist.library.empty-title": "No playlists yet",
  "playlist.library.empty-body": "Playlists you create are saved on this device.",
  "playlist.library.create": "New playlist",
  // Row context menu (kebab `•••` + right-click; playlist context menu.svg
  // grammar — full 8-entry port is the wave-c context-menu lane's).
  "playlist.library.menu": "Playlist actions",
  "playlist.library.menu.play": "Play",
  "playlist.library.menu.remove": "Remove",
  // Create-playlist dialog (new playlist.svg).
  "playlist.new.title": "New playlist",
  "playlist.new.close": "Close",
  "playlist.new.details": "Playlist details",
  "playlist.new.details.helper": "The playlist will be saved and displayed with this title and thumbnail.",
  "playlist.new.name-placeholder": "e.g., Favorite Videos",
  "playlist.new.thumbnail": "Playlist thumbnail",
  "playlist.new.thumbnail.hint": "Thumbnails are not exposed by this build yet.",
  "playlist.new.files": "Select files",
  "playlist.new.files.helper": "These files will be added to the playlist.",
  "playlist.new.drop": "Drag files into the box or click here to select files.",
  "playlist.new.browse": "Select files",
  "playlist.new.cancel": "Cancel",
  "playlist.new.save": "Save",
  // Row meta (saved file names count).
  "playlist.library.items-suffix": "items",
} as const;
