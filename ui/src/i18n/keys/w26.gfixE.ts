// w26.gfix-E lane keys (playlist surfaces fix cycle, issue #62) — copy with
// no existing catalog entry. Registered into ui/src/i18n/messages.ts by the
// Lead after the wave merge (contract §4.5); until then the playlist screens
// resolve these through the seam-first resolver (see
// ui/src/screens/playlist/options/useOptionsMsg.ts — this key file is merged
// into that lane catalog), so the copy renders identically before and after.
//
// Copy provenance: `file context menu.svg` carries REAL outlined text (not
// placeholders — OCR of the render). Labels are frame-verbatim (Title Case —
// the frames win over the general sentence-case rule, contract §3, same as
// the w26.b10 option-menu rows).
export const keys = {
  // File-row context menu (file context menu.svg — 12 icon rows, two-tone
  // #8F8F8F + #AB6314). Rows whose frame copy matches already-registered
  // keys reuse them (playlist.queue.menu.play / .favorite); the rest carry
  // this lane's frame-verbatim Title Case copy (the c6 queue keys are
  // sentence-case placeholders — the file menu frame has REAL labels).
  "playlist.file.menu.title": "File actions",
  "playlist.file.menu.mark-watched": "Mark as Watched",
  "playlist.file.menu.stream": "Stream",
  "playlist.file.menu.convert": "Convert",
  "playlist.file.menu.add-to-queue": "Add to Queue",
  "playlist.file.menu.add-to-playlist": "Add to Playlist",
  "playlist.file.menu.show-in-explorer": "Show in File Explorer",
  "playlist.file.menu.go-to-source": "Go to Source",
  "playlist.file.menu.add-subtitles": "Add Subtitle Tracks",
  "playlist.file.menu.add-audio": "Add Audio Tracks",
  "playlist.file.menu.properties": "Properties & Metadata",
} as const;
