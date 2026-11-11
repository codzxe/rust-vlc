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

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { usePlaylist, type PlaylistEntry } from "../PlaylistProvider";
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
}: {
  children: ReactNode;
  initialSortKey?: PlaylistSortKey;
  initialViewMode?: PlaylistViewMode;
  resolveMeta?: ResolveEntryMeta;
}) {
  const playlist = usePlaylist();
  const [sortKey, setSortKey] = useState<PlaylistSortKey>(initialSortKey);
  const [viewMode, setViewMode] = useState<PlaylistViewMode>(initialViewMode);

  // Presentation order only — the engine queue is never mutated by sorting.
  const sortedEntries = useMemo(
    () => sortPlaylistEntries(playlist.entries, sortKey, resolveMeta ?? (() => undefined)),
    [playlist.entries, sortKey, resolveMeta],
  );

  const metaOf = useCallback(
    (entry: PlaylistEntry): PlaylistEntryMeta | undefined => resolveMeta?.(entry),
    [resolveMeta],
  );

  const value = useMemo<PlaylistOptionsValue>(
    () => ({ sortKey, setSortKey, viewMode, setViewMode, sortedEntries, resolveMeta, metaOf }),
    [sortKey, viewMode, sortedEntries, resolveMeta, metaOf],
  );

  return <PlaylistOptionsContext.Provider value={value}>{children}</PlaylistOptionsContext.Provider>;
}

export function usePlaylistOptions(): PlaylistOptionsValue {
  const ctx = useContext(PlaylistOptionsContext);
  if (ctx === null) throw new Error("usePlaylistOptions requires a PlaylistOptionsProvider ancestor");
  return ctx;
}
