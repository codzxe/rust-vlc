// w26.b10 — "Sorting options" trigger + menu (`Sorting Options.svg`):
// the frame's 16 rows, no icons, no shortcuts, row 1 ("Unwatched First")
// checked by default. Labels are frame-verbatim (keys/w26.b10.ts).

import type { ReactElement } from "react";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";
import { PlaylistOptionsMenu } from "./PlaylistOptionsMenu";
import { SORT_KEYS, sortLabelKey, type PlaylistSortKey } from "./sorting";
import { useOptionsMsg, type OptionsLaneKey } from "./useOptionsMsg";

export function SortingOptionsMenu(): ReactElement {
  const t = useOptionsMsg();
  const { sortKey, setSortKey } = usePlaylistOptions();
  return (
    <PlaylistOptionsMenu<OptionsLaneKey | `playlist.options.sort.${PlaylistSortKey}`>
      entries={SORT_KEYS.map((key) => ({ id: key, labelKey: sortLabelKey(key) }))}
      value={sortKey}
      onSelect={(id) => setSortKey(id as PlaylistSortKey)}
      resolveLabel={t}
      menuLabel={t("playlist.options.sort")}
      triggerLabel={t("playlist.options.sort")}
      triggerIcon="sort"
      className="lumen-plo--sorting"
    />
  );
}
