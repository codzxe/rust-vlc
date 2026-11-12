// w26.b10 — playlist options layer public exports. Integration recipe for
// the Lead lives in docs/waves/w26/logs/w26.b10.md; in short:
//
//   <PlaylistProvider>            (existing — engine queue scope)
//     <PlaylistOptionsProvider>   (sort/view state + meta seam)
//       …panel header:   <PlaylistOptionsToolbar />
//       …panel body:     <PlaylistOptionsViews onEntryMenu={…} />
//     </PlaylistOptionsProvider>
//   </PlaylistProvider>
//
// w26.f06 — SHIPPED on /playlist: PlaylistScreen mounts the toolbar + views
// in the opened-playlist browse view, with a LOCAL `entries` scope
// (saved-playlist item names) instead of an engine PlaylistProvider.
export { PlaylistOptionsProvider, usePlaylistOptions, DEFAULT_VIEW_MODE } from "./PlaylistOptionsProvider";
export type { PlaylistOptionsValue, PlaylistViewMode } from "./PlaylistOptionsProvider";
export { PlaylistOptionsToolbar, PlaylistOptionsViews } from "./PlaylistOptionsToolbar";
export type { PlaylistOptionsToolbarProps } from "./PlaylistOptionsToolbar";
export { PlaylistOptionsMenu } from "./PlaylistOptionsMenu";
export type { OptionsMenuEntry, PlaylistOptionsMenuProps } from "./PlaylistOptionsMenu";
export { SortingOptionsMenu } from "./SortingOptionsMenu";
export { ViewOptionsMenu } from "./ViewOptionsMenu";
export { ThumbnailGrid } from "./ThumbnailGrid";
export { MediaList, MEDIA_ROW_HEIGHT } from "./MediaList";
export type { MediaListProps } from "./MediaList";
export {
  SORT_KEYS,
  DEFAULT_SORT_KEY,
  sortLabelKey,
  sortPlaylistEntries,
  sortUsesMeta,
  entryDisplayTitle,
  formatDurationClock,
} from "./sorting";
export type { PlaylistSortKey, PlaylistEntryMeta, ResolveEntryMeta } from "./sorting";
export { useOptionsMsg } from "./useOptionsMsg";
export type { OptionsLaneKey } from "./useOptionsMsg";
