// w26.gfix-E — the FILE-ROW context menu (`file context menu.svg`, 265×584
// r10 #191919@95: 12 rows × 48px, two-tone #8F8F8F + #AB6314 icons, labels
// #8F8F8F — OCR-verbatim Title Case, contract §3 "frames win"). Mounted by
// the /playlist browse media view (w26.g18 P2-1: the media.svg hover "•••"
// needs a real seam; w26.g20 P2-4: this frame had no implementer).
//
// Honest wiring per contract §5 (the queueRowMenu / library-row precedents):
// rows carry REAL seams only — Mark as Watched (the client-local watched
// override in PlaylistOptionsProvider), Convert + Properties & Metadata (the
// real /convert + /media-info routes, the queue menu's Media-info precedent),
// Add to Playlist (the local playlistsStore) and Add Subtitle Tracks (the
// AddSubtitlesDialog scoped to the one file, Folder-2's "To This File"
// state). No-seam rows render disabled — Play (no catalog command for local
// names), Favorite (no favorites store), Stream / Add to Queue / Show in
// File Explorer / Go to Source (no per-item stream, queue or OS-reveal
// seam — saved items are names only, never engine ids) and Add Audio Tracks
// (no per-item track-add command). Never theater.

import type { MenuEntry } from "../../../components/menu";
import type { SavedPlaylist } from "../playlistsStore";
import type { PlaylistEntry } from "../PlaylistProvider";

/** Everything the 12 frame rows need from the host. */
export interface FileRowMenuInput {
  /** The file row the menu was opened on. */
  entry: PlaylistEntry;
  /** Watched truth (the provider's merged meta — checkbox face). */
  watched: boolean;
  /** Saved playlists OTHER than the one being browsed (submenu targets). */
  otherPlaylists: readonly SavedPlaylist[];
  /** Mark as Watched — the real client-local watched override. */
  onSetWatched: (entry: PlaylistEntry, watched: boolean) => void;
  /** Add to Playlist — the real local store append. */
  onAddToPlaylist: (playlistId: string, name: string) => void;
  /** Add Subtitle Tracks — the AddSubtitlesDialog scoped to this file. */
  onAddSubtitles: (entry: PlaylistEntry) => void;
  /** In-app navigation (Convert / Properties & Metadata routes). */
  navigate: (to: string) => void;
}

/** Submenu labelKey for one saved-playlist target (dynamic titles resolve
 * through fileMenuResolveLabel's catalog fallback, the c6 lane precedent). */
export const playlistTargetKey = (playlistId: string): string => `playlist.target.${playlistId}`;

/**
 * Label resolver for the file-row menu: lane keys through the options seam,
 * then the DYNAMIC Add-to-Playlist submenu targets (saved-playlist titles —
 * runtime data, not i18n keys; the PlaylistPanel c6-catalog fallback
 * precedent), then the key itself (dev/sandbox, identityResolveLabel rule).
 */
export function fileMenuResolveLabel(
  t: (key: string) => string,
  targets: ReadonlyMap<string, string>,
): (key: string) => string {
  return (key: string): string => (t(key) as string | undefined) ?? targets.get(key) ?? key;
}

/**
 * The 12 frame rows in pinned order: Play · Mark as Watched · Favorite ·
 * Stream · Convert · Add to Queue · Add to Playlist · Show in File Explorer ·
 * Go to Source · Add Subtitle Tracks · Add Audio Tracks · Properties &
 * Metadata. Icons all carry the frame's two-tone accent (`iconAccent`).
 */
export function fileRowMenu({
  entry,
  watched,
  otherPlaylists,
  onSetWatched,
  onAddToPlaylist,
  onAddSubtitles,
  navigate,
}: FileRowMenuInput): MenuEntry<string>[] {
  return [
    // 1 — Play (frame: orange play triangle). No catalog command exists for
    // local saved names — disabled-honest (LibraryRow's Play precedent).
    { id: "play", labelKey: "playlist.queue.menu.play", icon: "play", iconAccent: true, disabled: true },
    // 2 — Mark as Watched (frame: orange check glyph) → the provider's real
    // watched override; the checkbox face is the merged watched truth.
    {
      id: "mark-watched",
      labelKey: "playlist.file.menu.mark-watched",
      icon: "check",
      iconAccent: true,
      kind: "checkbox",
      checked: watched,
      onSelect: () => onSetWatched(entry, !watched),
    },
    // 3 — Favorite (frame: orange heart). No favorites store — disabled.
    { id: "favorite", labelKey: "playlist.queue.menu.favorite", icon: "favorite", iconAccent: true, disabled: true },
    // 4 — Stream (frame: two-tone broadcast). No per-item stream seam.
    { id: "stream", labelKey: "playlist.file.menu.stream", icon: "network-stream", iconAccent: true, disabled: true },
    // 5 — Convert (frame: two-tone convert glyph) → the real /convert route
    // (the queue menu's Media-info navigation precedent).
    {
      id: "convert",
      labelKey: "playlist.file.menu.convert",
      icon: "convert",
      iconAccent: true,
      onSelect: () => navigate("/convert"),
    },
    // 6 — Add to Queue (frame: orange pill + lines). Local names never carry
    // engine ids (no dispatch may use them) — disabled.
    { id: "add-to-queue", labelKey: "playlist.file.menu.add-to-queue", icon: "queue", iconAccent: true, disabled: true },
    // 7 — Add to Playlist (frame: playlist glyph) → the real local store
    // append, one submenu row per OTHER saved playlist; with no other
    // playlist there is no target — the row renders disabled-honest.
    {
      id: "add-to-playlist",
      labelKey: "playlist.file.menu.add-to-playlist",
      icon: "playlist",
      iconAccent: true,
      disabled: otherPlaylists.length === 0,
      submenu: otherPlaylists.map((playlist) => ({
        id: `add-to-${playlist.id}`,
        labelKey: playlistTargetKey(playlist.id),
        icon: "playlist",
        onSelect: () => onAddToPlaylist(playlist.id, entry.title ?? ""),
      })),
    },
    // 8 — Show in File Explorer (frame: folder + magnifier). No OS
    // reveal seam (names only, no paths) — disabled.
    {
      id: "show-in-explorer",
      labelKey: "playlist.file.menu.show-in-explorer",
      icon: "open-folder",
      iconAccent: true,
      disabled: true,
    },
    // 9 — Go to Source (frame: arrow out). Saved items carry no source
    // (source: null) — disabled.
    { id: "go-to-source", labelKey: "playlist.file.menu.go-to-source", icon: "open-file", iconAccent: true, disabled: true },
    // 10 — Add Subtitle Tracks (frame: subtitle card + orange dashes) → the
    // AddSubtitlesDialog scoped to this one file (Folder-2 "To This File").
    {
      id: "add-subtitles",
      labelKey: "playlist.file.menu.add-subtitles",
      icon: "subtitles",
      iconAccent: true,
      onSelect: () => onAddSubtitles(entry),
    },
    // 11 — Add Audio Tracks (frame: two-tone waveform). No per-item
    // track-add command — disabled.
    { id: "add-audio", labelKey: "playlist.file.menu.add-audio", icon: "audio", iconAccent: true, disabled: true },
    // 12 — Properties & Metadata (frame: info circle + orange dot) → the
    // real /media-info route (queue menu's Media-info precedent).
    {
      id: "properties",
      labelKey: "playlist.file.menu.properties",
      icon: "info",
      iconAccent: true,
      onSelect: () => navigate("/media-info"),
    },
  ];
}
