// w26.b10 — the Lead's two mount points (integration recipe in
// docs/waves/w26/logs/w26.b10.md):
//
//   <PlaylistOptionsToolbar />   → the panel header's meta row (sort + view)
//   <PlaylistOptionsViews />     → the panel body (grid | list switch)
//
// Both read `usePlaylist()` + `usePlaylistOptions()` — zero props; mount
// them inside <PlaylistProvider><PlaylistOptionsProvider>…</></>.

import type { ReactElement } from "react";
import { MediaList, type MediaListProps } from "./MediaList";
import { SortingOptionsMenu } from "./SortingOptionsMenu";
import { ThumbnailGrid } from "./ThumbnailGrid";
import { ViewOptionsMenu } from "./ViewOptionsMenu";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";

/** Meta-row controls: the sorting trigger + the view trigger side by side. */
export function PlaylistOptionsToolbar(): ReactElement {
  return (
    <div className="lumen-plo-toolbar">
      <SortingOptionsMenu />
      <ViewOptionsMenu />
    </div>
  );
}

/** Body switch: `View Options.svg` grid mode → ThumbnailGrid, else MediaList. */
export function PlaylistOptionsViews(props: MediaListProps = {}): ReactElement {
  const { viewMode } = usePlaylistOptions();
  return viewMode === "grid" ? <ThumbnailGrid /> : <MediaList {...props} />;
}
