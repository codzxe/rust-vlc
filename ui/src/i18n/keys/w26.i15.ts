// w26.i15 lane keys (fixer, wave I — h11/h12 closing: the staged frame rows
// for library / network / recording / extensions, per the Lead's wave-I fix
// directive: the frames' rows render with the honest-empty pattern in the
// frame's own visual grammar (staged-inert, the Remotes precedent), never
// dead controls that look alive). Registered into ui/src/i18n/messages.ts by
// the Lead after the wave merge (contract §4.5); until then the areas
// resolve these via the lane seam (areas/staged.tsx useStagedMsg).
//
// Copy is frame-verbatim (tools-settings.md §C9/§C10/§C12/§C13 + the SVG
// text ids): section headers title case (every sibling section), row labels
// sentence case exactly as the frames spell them, control values verbatim.
export const keys = {
  // --- Section titles (frame headers, title case like every sibling) ------
  "settings.section.library.opening-files": "Opening Files",
  "settings.section.library.metadata-artwork": "Metadata & Artwork",
  "settings.section.network.buffering": "Buffering",
  "settings.section.network.proxy": "Proxy",
  "settings.section.network.security": "Security",
  "settings.section.recording.casting": "Casting",
  "settings.section.recording.streaming": "Streaming",

  // --- Row labels — library (§C9 y83/y356) ----------------------------------
  "settings.library.single-window": "Only allow one VLC window",
  "settings.library.file-manager": "Apply to files opened from the file manager",
  "settings.library.playlist-add": "Add to the playlist instead of replacing it",
  "settings.library.folder-open": "When opening a folder",
  "settings.library.show-hidden": "Show hidden files",
  "settings.library.fetch-metadata": "Fetch titles and artwork",
  "settings.library.use-media-library": "Use the media library",

  // --- Row labels — network (§C10 y83/y380/y535) -----------------------------
  "settings.network.buffer-size": "Buffering",
  "settings.network.local-files": "Local files",
  "settings.network.discs": "Discs",
  "settings.network.network-streams": "Network streams",
  "settings.network.live-capture": "Live capture",
  "settings.network.http-proxy": "HTTP proxy",
  "settings.network.http-proxy-password": "HTTP proxy password",
  "settings.network.verify-certificates": "Verify certificates against the system trust store",
  "settings.network.cipher-preference": "Cipher preference",

  // --- Row labels — recording (§C12 y83/y232/y379) ---------------------------
  // recording.directory ("Save recordings to") + the generated section title
  // ("Recording") are already-registered msgids — not duplicated here.
  "settings.recording.without-reencoding": "Record without re-encoding when possible",
  "settings.recording.discover-cast": "Discover cast devices",
  "settings.recording.cast-quality": "Cast quality",
  "settings.recording.play-locally": "Play locally while streaming",
  "settings.recording.include-audio": "Include audio",
  "settings.recording.include-video": "Include video",
  "settings.recording.include-subtitles": "Include subtitles",
  "settings.recording.encoding-preset": "Encoding preset",
  "settings.recording.tuning": "Tuning",

  // --- Control values (frame-verbatim dropdown/input text) ------------------
  "settings.library.value.folder.include-subfolders": "Include subfolders",
  "settings.library.value.metadata.file-only": "From the file only",
  "settings.network.value.buffer.normal": "Normal",
  "settings.network.value.proxy.system": "System proxy",
  "settings.network.value.cipher.balanced": "Balanced",
  "settings.recording.value.cast.automatic": "Automatic",
  "settings.recording.value.preset.medium": "Medium",
  "settings.recording.value.tuning.none": "None",
  "settings.recording.value.directory.default": "~/Videos",

  // --- Staged-area tooltips (one per awaited seam; the honest affordance) --
  "settings.library.staged":
    "Staged — these rows activate when the library settings schema ships",
  "settings.network.staged":
    "Staged — these rows activate when the network settings schema ships",
  "settings.recording.staged": "Staged — these rows activate when recording ships in this build",
  "settings.extensions.staged":
    "Staged — add-on management activates when the extensions feature ships",
  "settings.advanced.staged": "Staged — advanced options open once this area carries them",

  // --- w26.i15 relaunch (h11 #5): the hotkeys editor dialog (controls) -----
  "settings.hotkeys.dialog.close": "Close",
} as const;
