// w26.b10 — "Sorting options" trigger + menu (`Sorting Options.svg`):
// the frame's 16 rows, no icons, no shortcuts, row 1 ("Unwatched First")
// checked by default. Labels are frame-verbatim (keys/w26.b10.ts).
//
// w26.k15 (j15 P3-11): metadata-dependent rows render a quiet honest-
// degradation hint ("Needs media metadata") while nothing can influence
// their order — no engine resolveMeta seam AND no local watched marks
// (PlaylistOptionsProvider.metaSeamWired). The frame pins plain rows; the
// hint appears only in the unwired state (harness/frame mounts wire a
// resolver, so comparison shots stay frame-pure).

import type { ReactElement } from "react";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";
import { PlaylistOptionsMenu } from "./PlaylistOptionsMenu";
import { SORT_KEYS, sortLabelKey, sortUsesMeta, type PlaylistSortKey } from "./sorting";
import { useOptionsMsg, type OptionsLaneKey } from "./useOptionsMsg";

export function SortingOptionsMenu({ align = "start" }: { align?: "start" | "end" } = {}): ReactElement {
  const t = useOptionsMsg();
  const { sortKey, setSortKey, metaSeamWired } = usePlaylistOptions();
  return (
    <PlaylistOptionsMenu<OptionsLaneKey | `playlist.options.sort.${PlaylistSortKey}`>
      entries={SORT_KEYS.map((key) => ({
        id: key,
        labelKey: sortLabelKey(key),
        hint:
          sortUsesMeta(key) && !metaSeamWired
            ? t("playlist.options.sort.needs-meta")
            : undefined,
      }))}
      value={sortKey}
      onSelect={(id) => setSortKey(id as PlaylistSortKey)}
      resolveLabel={t}
      menuLabel={t("playlist.options.sort")}
      triggerLabel={t("playlist.options.sort")}
      triggerIcon="sort"
      align={align}
      className="lumen-plo--sorting"
    />
  );
}
