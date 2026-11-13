// w26.b10 — playlist sorting model (`Sorting Options.svg`, 16 frame rows).
//
// PURE functions only (no React) so the provider, the menus and the tests
// share one implementation. Engine model note (doc 03 §4 PlaylistItem): the
// wire model carries id / title / source / duration — artist, album, track,
// disc, watched and add-date metadata are NOT in the catalog yet. Those
// arrive through the `PlaylistEntryMeta` seam (PlaylistOptionsProvider), so
// every comparator degrades HONESTLY: unknown metadata never reorders the
// queue (it sorts last, stably) instead of pretending a value.
//
// Natural order (the 2026-11-09 v2 flow brief's "natural sort" fix, kept by
// the W25 spec): titles/URIs compare through `naturalCompare` — "Episode 2"
// before "Episode 10", never the lexical "1, 10, 2".

import { naturalCompare } from "../../../components/list/natural";
import type { PlaylistEntry } from "../PlaylistProvider";
import { sourceLocation } from "../queue";

/** The 16 sort options, in `Sorting Options.svg` row order (frame rows 1–16). */
export const SORT_KEYS = [
  "unwatched-first",
  "watched-first",
  "newest-first",
  "oldest-first",
  "title-asc",
  "title-desc",
  "artist-asc",
  "artist-desc",
  "album-asc",
  "album-desc",
  "track-number-asc",
  "track-number-desc",
  "uri-asc",
  "uri-desc",
  "disc-number-asc",
  "disc-number-desc",
] as const;

export type PlaylistSortKey = (typeof SORT_KEYS)[number];

/** Frame default: row 1 "Unwatched First" is the checked row. */
export const DEFAULT_SORT_KEY: PlaylistSortKey = "unwatched-first";

/** i18n key for a sort option (registered by the Lead; see keys/w26.b10.ts). */
export const sortLabelKey = (key: PlaylistSortKey): `playlist.options.sort.${PlaylistSortKey}` =>
  `playlist.options.sort.${key}`;

/**
 * Per-entry metadata seam. All fields optional/undefined = engine has not
 * reported (honest unknown). `watchedProgress` is 0..1 — the media view's
 * orange tile bar (media.svg "Rectangle 12").
 */
export interface PlaylistEntryMeta {
  readonly watched?: boolean;
  readonly watchedProgress?: number;
  readonly artist?: string;
  readonly album?: string;
  readonly trackNumber?: number;
  readonly discNumber?: number;
  /** Add time (epoch ms); absent → queue position is the honest proxy. */
  readonly addedAt?: number;
}

/** Resolver seam — the app shell wires engine metadata here (handoff). */
export type ResolveEntryMeta = (entry: PlaylistEntry) => PlaylistEntryMeta | undefined;

/** Display title for a media row / card: title → source basename → unknown. */
export function entryDisplayTitle(entry: PlaylistEntry, unknownTitle: string): string {
  if (entry.title !== null && entry.title !== "") return entry.title;
  const location = sourceLocation(entry.source);
  if (location !== null) return location.split(/[\\/]/).filter(Boolean).pop() ?? location;
  return unknownTitle;
}

/** `00:02:10` — zero-padded h:mm:ss (thumbnail.svg footer timecode). */
export function formatDurationClock(seconds: number | null): string | null {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) return null;
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

/** True when the sort key needs the metadata seam. w26.k15 (j15 P3-11): the
 * sorting menu renders its "Needs media metadata" hint on these rows while
 * the seam cannot influence the order (PlaylistOptionsProvider.
 * metaSeamWired === false) — the hint is real, not aspirational. */
export const sortUsesMeta = (key: PlaylistSortKey): boolean =>
  key.startsWith("artist-") ||
  key.startsWith("album-") ||
  key.startsWith("track-") ||
  key.startsWith("disc-") ||
  key === "unwatched-first" ||
  key === "watched-first";

interface Decorated {
  readonly entry: PlaylistEntry;
  readonly index: number;
  readonly meta: PlaylistEntryMeta | undefined;
}

/** Unknown-first comparator: undefined metadata sorts AFTER known. */
function knownFirst(a: string | number | undefined, b: string | number | undefined): number {
  if (a === undefined && b === undefined) return 0;
  if (a === undefined) return 1;
  if (b === undefined) return -1;
  return 0;
}

/** Sort key = the DISPLAYED title (title → source basename → ""): rows sort
 * the way they read (media.svg titles are source basenames). */
const titleOf = (entry: PlaylistEntry): string => entryDisplayTitle(entry, "");

const uriOf = (entry: PlaylistEntry): string => sourceLocation(entry.source) ?? "";

/** Key comparator over decorated entries. Deterministic: ties break by the
 * original queue index (stability is total, never engine-order roulette). */
function compareDecorated(a: Decorated, b: Decorated, key: PlaylistSortKey): number {
  switch (key) {
    case "unwatched-first":
    case "watched-first": {
      // Watched = the meta seam reports true. Unknown (undefined) counts as
      // unwatched — honest: nothing has reported it watched.
      const aw = a.meta?.watched === true;
      const bw = b.meta?.watched === true;
      if (aw !== bw) return key === "unwatched-first" ? (aw ? 1 : -1) : aw ? -1 : 1;
      return a.index - b.index;
    }
    case "newest-first":
    case "oldest-first": {
      // addedAt when the seam reports it; otherwise queue position is the
      // honest add-order proxy (playlist.changed appends in add order).
      const at = a.meta?.addedAt ?? a.index;
      const bt = b.meta?.addedAt ?? b.index;
      return key === "newest-first" ? bt - at : at - bt;
    }
    case "title-asc":
    case "title-desc": {
      const d = naturalCompare(titleOf(a.entry), titleOf(b.entry));
      return key === "title-asc" ? d : -d || a.index - b.index;
    }
    case "uri-asc":
    case "uri-desc": {
      const d = naturalCompare(uriOf(a.entry), uriOf(b.entry));
      return key === "uri-asc" ? d : -d || a.index - b.index;
    }
    case "artist-asc":
    case "artist-desc":
    case "album-asc":
    case "album-desc": {
      const field = key.startsWith("artist") ? "artist" : "album";
      const av = a.meta?.[field];
      const bv = b.meta?.[field];
      const unknown = knownFirst(av, bv);
      if (unknown !== 0) return unknown;
      if (av === undefined) return a.index - b.index;
      const d = naturalCompare(av, bv ?? "");
      return key.endsWith("asc") ? d || a.index - b.index : -d || a.index - b.index;
    }
    case "track-number-asc":
    case "track-number-desc":
    case "disc-number-asc":
    case "disc-number-desc": {
      const field = key.startsWith("track") ? "trackNumber" : "discNumber";
      const av = a.meta?.[field];
      const bv = b.meta?.[field];
      const unknown = knownFirst(av, bv);
      if (unknown !== 0) return unknown;
      if (av === undefined) return a.index - b.index;
      const d = av - (bv ?? av);
      return key.endsWith("asc") ? d || a.index - b.index : -d || a.index - b.index;
    }
  }
}

/**
 * Sorted copy of `entries` under `key` (input never mutated; ties keep queue
 * order). The engine stays the source of truth — this is presentation order
 * only; `playlist.move` dispatches are NOT fired.
 */
export function sortPlaylistEntries(
  entries: readonly PlaylistEntry[],
  key: PlaylistSortKey,
  resolveMeta: ResolveEntryMeta = () => undefined,
): readonly PlaylistEntry[] {
  const decorated: Decorated[] = entries.map((entry, index) => ({
    entry,
    index,
    meta: resolveMeta(entry),
  }));
  return decorated
    .sort((a, b) => {
      const d = compareDecorated(a, b, key);
      return d !== 0 ? d : a.index - b.index;
    })
    .map((d) => d.entry);
}
