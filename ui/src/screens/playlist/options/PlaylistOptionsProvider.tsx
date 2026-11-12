// w26.b10 — playlist options state container: sort key (`Sorting Options.svg`)
// + view mode (`View Options.svg`) + the engine-metadata seam the sort
// comparators and the media views consume. Mounts INSIDE PlaylistProvider
// (it reads the queue through `usePlaylist`) and AROUND the playlist panel
// body — see the integration recipe in docs/waves/w26/logs/w26.b10.md.
//
// HONESTY: `resolveMeta` defaults to "no metadata reported" — watched bars,
// artist/album/track/disc sorts and add-date ordering light up when the app
// shell wires the seam (engine catalog handoff); until then the presentation
// degrades honestly instead of inventing values.
//
// w26.f06 — LOCAL DATA SCOPE: `entries` overrides the engine queue. The
// /playlist library page browses a SAVED playlist (playlistsStore itemNames —
// names only, no engine ids); a nested engine PlaylistProvider would let
// `playlist.changed` events contaminate that view, so the provider reads the
// engine scope OPTIONALLY and sorts the local list when given one.

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { usePlaylistOptional, type PlaylistEntry } from "../PlaylistProvider";
import {
  DEFAULT_SORT_KEY,
  sortPlaylistEntries,
  type PlaylistEntryMeta,
  type PlaylistSortKey,
  type ResolveEntryMeta,
} from "./sorting";

/** View modes (`View Options.svg`: grid-icon row + checked list-icon row). */
export type PlaylistViewMode = "grid" | "list";

/** Frame default: "List View" is the checked row (View Options.svg row 2). */
export const DEFAULT_VIEW_MODE: PlaylistViewMode = "list";

export interface PlaylistOptionsValue {
  readonly sortKey: PlaylistSortKey;
  setSortKey(key: PlaylistSortKey): void;
  readonly viewMode: PlaylistViewMode;
  setViewMode(mode: PlaylistViewMode): void;
  /** Entries ordered by the current sort (presentation order only). */
  readonly sortedEntries: readonly PlaylistEntry[];
  /** Engine metadata seam (watched state, artist/album/track/disc, add date).
   * Undefined = no metadata reported (the honest default). */
  readonly resolveMeta: ResolveEntryMeta | undefined;
  /** Meta lookup bound to entry identity — stable for consumers. */
  metaOf(entry: PlaylistEntry): PlaylistEntryMeta | undefined;
  /** w26.gfix-E — Mark as Watched: a client-local watched override merged
   * over `resolveMeta` (the file context menu's row 2). No engine command
   * exists (doc 05 §2.9 has no watched command) and nothing persists — the
   * row's watched state + the watched-first sorts are the real, visible
   * effect. */
  setWatched(entry: PlaylistEntry, watched: boolean): void;
}

const PlaylistOptionsContext = createContext<PlaylistOptionsValue | null>(null);

export function PlaylistOptionsProvider({
  children,
  /** Initial sort (tests / deep links); defaults to the frame default. */
  initialSortKey = DEFAULT_SORT_KEY,
  /** Initial view mode; defaults to the frame default ("List View"). */
  initialViewMode = DEFAULT_VIEW_MODE,
  /** Engine metadata seam — wire engine-reported values here (handoff). */
  resolveMeta,
  /** Local data scope (w26.f06): when set, THESE entries are the data
   * source instead of the engine queue (saved-playlist browse). No engine
   * PlaylistProvider is required in that mode. */
  entries,
}: {
  children: ReactNode;
  initialSortKey?: PlaylistSortKey;
  initialViewMode?: PlaylistViewMode;
  resolveMeta?: ResolveEntryMeta;
  entries?: readonly PlaylistEntry[];
}) {
  const enginePlaylist = usePlaylistOptional();
  const [sortKey, setSortKey] = useState<PlaylistSortKey>(initialSortKey);
  const [viewMode, setViewMode] = useState<PlaylistViewMode>(initialViewMode);
  // w26.gfix-E — client-local Mark-as-Watched overrides, keyed by entry id.
  const [watchedOverrides, setWatchedOverrides] = useState<ReadonlyMap<number, boolean>>(
    () => new Map(),
  );

  // The EFFECTIVE metadata seam: the local watched override merged over the
  // engine-reported meta (override wins; unmarking also clears the partial
  // progress bar — media.svg's watched states). Pure derivation, no clone of
  // the caller's resolveMeta.
  const resolveMetaEffective = useCallback(
    (entry: PlaylistEntry): PlaylistEntryMeta | undefined => {
      const base = resolveMeta?.(entry);
      const override = watchedOverrides.get(entry.id);
      if (override === undefined) return base;
      return override
        ? { ...base, watched: true }
        : { ...base, watched: false, watchedProgress: 0 };
    },
    [resolveMeta, watchedOverrides],
  );

  // Presentation order only — the engine queue is never mutated by sorting.
  // Local scope (entries prop) wins over the engine queue.
  const sourceEntries = useMemo(
    () => entries ?? enginePlaylist?.entries ?? [],
    [entries, enginePlaylist],
  );
  const sortedEntries = useMemo(
    () => sortPlaylistEntries(sourceEntries, sortKey, resolveMetaEffective),
    [sourceEntries, sortKey, resolveMetaEffective],
  );

  const metaOf = useCallback(
    (entry: PlaylistEntry): PlaylistEntryMeta | undefined => resolveMetaEffective(entry),
    [resolveMetaEffective],
  );

  const setWatched = useCallback((entry: PlaylistEntry, watched: boolean): void => {
    setWatchedOverrides((prev) => {
      const next = new Map(prev);
      next.set(entry.id, watched);
      return next;
    });
  }, []);

  const value = useMemo<PlaylistOptionsValue>(
    () => ({
      sortKey,
      setSortKey,
      viewMode,
      setViewMode,
      sortedEntries,
      resolveMeta,
      metaOf,
      setWatched,
    }),
    [sortKey, viewMode, sortedEntries, resolveMeta, metaOf, setWatched],
  );

  return <PlaylistOptionsContext.Provider value={value}>{children}</PlaylistOptionsContext.Provider>;
}

export function usePlaylistOptions(): PlaylistOptionsValue {
  const ctx = useContext(PlaylistOptionsContext);
  if (ctx === null) throw new Error("usePlaylistOptions requires a PlaylistOptionsProvider ancestor");
  return ctx;
}
