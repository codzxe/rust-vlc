// w26.b11 lane keys (playlist subtitle flows) — the add-subtitles dialog
// copy (design/figma: "Add Subtitles to Playlist/Folder*.svg",
// "subtitle one.svg", "subtitle multiple.svg", "sub filename.svg") that has
// no existing catalog entry. Registered into ui/src/i18n/messages.ts by the
// Lead after the wave merge (contract §4.5); until then this lane resolves
// the keys through the seam-first resolver (see
// ui/src/screens/playlist/subtitles/useSubtitlesMsg.ts), so the copy renders
// identically before and after registration. Existing generic keys were
// grepped first — no duplicates (the dialog footers reuse this lane's own
// Cancel/Save pair until the Lead consolidates catalog-wide).
export const keys = {
  // Full-window overlay title + header close (Folder.svg "Add Subtitles").
  "playlist.subtitles.title": "Add Subtitles",
  "playlist.subtitles.close": "Close",
  // Destination scope (Folder.svg "To This Folder" / Folder-2/3 "To This
  // File"; the playlist-wide third option composes the same pill).
  "playlist.subtitles.destination": "Apply subtitles to",
  "playlist.subtitles.destination.playlist": "To This Playlist",
  "playlist.subtitles.destination.folder": "To This Folder",
  "playlist.subtitles.destination.file": "To This File",
  // Source scope (Folder/Folder-2 "From Computer", Folder-3 "From
  // OpenSubtitles").
  "playlist.subtitles.source": "Subtitle source",
  "playlist.subtitles.source.computer": "From Computer",
  "playlist.subtitles.source.online": "From OpenSubtitles",
  // Breadcrumb pill (Folder.svg "Videos / Shows / The Wand / S01").
  "playlist.subtitles.folder-path": "Subtitle folder path",
  // Online search controls (Folder-3: search field "The Wand S01E01" +
  // language pill "English").
  "playlist.subtitles.search": "Search subtitles",
  "playlist.subtitles.search.placeholder": "The Wand S01E01",
  "playlist.subtitles.language": "Subtitle language",
  "playlist.subtitles.language.english": "English",
  // Filename matching strategy (sub filename.svg option rows, verbatim).
  "playlist.subtitles.match": "Filename matching",
  "playlist.subtitles.match.off": "Off",
  "playlist.subtitles.match.any": "Any title",
  "playlist.subtitles.match.contains": "Contains title",
  "playlist.subtitles.match.extra": "Title with extra characters",
  "playlist.subtitles.match.exact": "Exact title match",
  // Candidate list a11y names (tabular count + suffix grammar reserved for
  // the host's own summary line — the frames pin no count caption).
  "playlist.subtitles.list-label": "Subtitle files",
  "playlist.subtitles.pairs-label": "Matched subtitle pairs",
  // Honest empty state (no candidates reported by the wired seam).
  "playlist.subtitles.empty-title": "No subtitle files found",
  "playlist.subtitles.empty-body": "Subtitle files that match your videos appear here.",
  // Footer (Folder.svg Frame 140/141: Cancel #1B1B1B, Save #371F06).
  "playlist.subtitles.cancel": "Cancel",
  "playlist.subtitles.save": "Save",
} as const;
