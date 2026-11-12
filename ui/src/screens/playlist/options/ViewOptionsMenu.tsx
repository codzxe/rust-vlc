// w26.b10 — "View options" trigger + menu (`View Options.svg`): two rows —
// grid icon "Grid View", list icon "List View" (checked in the frame). The
// trigger mirrors the active mode's icon (list while in list view).

import type { ReactElement } from "react";
import type { IconName } from "../../../components/icon/icons";
import { PlaylistOptionsMenu } from "./PlaylistOptionsMenu";
import { usePlaylistOptions, type PlaylistViewMode } from "./PlaylistOptionsProvider";
import { useOptionsMsg, type OptionsLaneKey } from "./useOptionsMsg";

interface ViewOption {
  readonly id: PlaylistViewMode;
  readonly labelKey: OptionsLaneKey;
  readonly icon: IconName;
}

const VIEW_MODES: readonly ViewOption[] = [
  { id: "grid", labelKey: "playlist.options.view.grid", icon: "grid" },
  { id: "list", labelKey: "playlist.options.view.list", icon: "list" },
];

export function ViewOptionsMenu({ align = "start" }: { align?: "start" | "end" } = {}): ReactElement {
  const t = useOptionsMsg();
  const { viewMode, setViewMode } = usePlaylistOptions();
  const activeIcon = viewMode === "grid" ? "grid" : "list";
  return (
    <PlaylistOptionsMenu
      entries={VIEW_MODES.map(({ id, labelKey, icon }) => ({ id, labelKey, icon }))}
      value={viewMode}
      onSelect={(id) => setViewMode(id as PlaylistViewMode)}
      resolveLabel={t}
      menuLabel={t("playlist.options.view")}
      triggerLabel={t("playlist.options.view")}
      triggerIcon={activeIcon}
      align={align}
      className="lumen-plo--view"
    />
  );
}
