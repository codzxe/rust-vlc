// File-open classification (issue #3 — the 4.H1 handoffs b + d): the
// app-layer rule that decides what "opening this path" MEANS on the
// wire. A media file opens a session (`session.open`); a playlist file
// loads the list (`playlist.load {path, append: false}` — parse lives
// engine-side in crates/playlists, the UI never parses; 4.D1's
// handoff). Both are catalog commands over the SAME transport — there
// is no second file-open mechanism.
//
// The Rust shell mirrors this exact rule (apps/desktop files.rs
// `is_playlist_path` + dispatch.rs `open_path_request`) for
// dialog/drag-drop acquisitions; one rule, two acquisition sides.

import type { CommandRequest, SourceSpec } from "./transport";

/**
 * Playlist file suffixes (the engine-side playlists crate's reader
 * surface: M3U/M3U8, PLS, XSPF, CUE). Anything else is media.
 */
export const PLAYLIST_FILE_EXTENSIONS: readonly string[] = [
  "m3u",
  "m3u8",
  "pls",
  "xspf",
  "cue",
];

/** True when a path points at a playlist file (case-insensitive suffix). */
export function isPlaylistPath(path: string): boolean {
  const dot = path.lastIndexOf(".");
  if (dot < 0 || dot === path.length - 1) return false;
  const extension = path.slice(dot + 1).toLowerCase();
  return PLAYLIST_FILE_EXTENSIONS.includes(extension);
}

/** What opening one source means: a session or a playlist load. */
export type OpenPathDecision =
  | { readonly kind: "media"; readonly source: SourceSpec }
  | { readonly kind: "playlist"; readonly path: string };

/** Classify a source: URLs are always media; files split by suffix. */
export function classifyOpenSource(source: SourceSpec): OpenPathDecision {
  if (source.kind === "url") return { kind: "media", source };
  return isPlaylistPath(source.path)
    ? { kind: "playlist", path: source.path }
    : { kind: "media", source };
}

/**
 * The wire request that opens `source` — the ONE funnel every
 * UI-originated file open goes through (dialog picks, pasted paths,
 * demo clips). Playlist files load the list (append: false — an open
 * replaces the queue); media files open a session.
 */
export function openPathRequest(
  source: SourceSpec,
  requestId: number,
): CommandRequest {
  const decision = classifyOpenSource(source);
  return decision.kind === "playlist"
    ? {
        requestId,
        cmd: "playlist.load",
        args: { path: decision.path, append: false },
      }
    : { requestId, cmd: "session.open", args: { source } };
}

/** Where the app routes after opening: the playlist screen or the player. */
export function openPathRoute(source: SourceSpec): "/playlist" | "/player" {
  return classifyOpenSource(source).kind === "playlist" ? "/playlist" : "/player";
}
