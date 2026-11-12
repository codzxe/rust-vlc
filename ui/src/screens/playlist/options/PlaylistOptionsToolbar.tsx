// w26.b10 — the Lead's two mount points (integration recipe in
// docs/waves/w26/logs/w26.b10.md):
//
//   <PlaylistOptionsToolbar />   → the panel header's meta row (sort + view)
//   <PlaylistOptionsViews />     → the panel body (grid | list switch)
//
// Both read `usePlaylistOptions()`; mount them inside a
// <PlaylistOptionsProvider> (w26.f06: an engine PlaylistProvider ancestor is
// only needed when the data source is the engine queue — a local `entries`
// scope works without one).
//
// w26.f06 — SHIPPED on /playlist: the toolbar row of the opened-playlist
// browse view (sort + view triggers) and the body switch below it.

import type { ReactElement } from "react";
import { MediaList, type MediaListProps } from "./MediaList";
import { SortingOptionsMenu } from "./SortingOptionsMenu";
import { ThumbnailGrid } from "./ThumbnailGrid";
import { ViewOptionsMenu } from "./ViewOptionsMenu";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";

/** Popup alignment forwarded to both menus (e.g. "end" in a right-aligned
 * toolbar row so the 265px panels hug the row's right edge). */
export interface PlaylistOptionsToolbarProps {
  align?: "start" | "end";
}

/** Meta-row controls: the sorting trigger + the view trigger side by side. */
export function PlaylistOptionsToolbar({ align = "start" }: PlaylistOptionsToolbarProps = {}): ReactElement {
  return (
    <div className="lumen-plo-toolbar">
      <SortingOptionsMenu align={align} />
      <ViewOptionsMenu align={align} />
    </div>
  );
}

/** Body switch: `View Options.svg` grid mode → ThumbnailGrid, else MediaList. */
export function PlaylistOptionsViews(props: MediaListProps = {}): ReactElement {
  const { viewMode } = usePlaylistOptions();
  return viewMode === "grid" ? <ThumbnailGrid /> : <MediaList {...props} />;
}
