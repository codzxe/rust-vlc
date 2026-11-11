// w26.b11 — the filename-matching strategy control (`sub filename.svg`):
// the shared Dropdown (w26 grammar twin — trigger r6 with ring + chevron;
// open list r6 #191919 + drop shadow; selected row #1F1F1F + orange check)
// pinned to the frame's scale: 48px two-layer trigger (#090909 ring over a
// #191919 field), 44px option rows, 362px-wide list (scoped in
// subtitles.css). Options verbatim: Off / Any title / Contains title /
// Title with extra characters / Exact title match; the frame's checked row
// is "Title with extra characters" (matching.DEFAULT_MATCH_MODE).

import type { ReactElement } from "react";
import { Dropdown } from "../../../components/menu";
import { MATCH_MODES, type MatchMode } from "./matching";
import { useSubtitlesMsg, type SubtitlesLaneKey } from "./useSubtitlesMsg";

/** Strategy id → lane key (frame row labels, verbatim). */
const LABEL_KEY: Readonly<Record<MatchMode, SubtitlesLaneKey>> = {
  off: "playlist.subtitles.match.off",
  any: "playlist.subtitles.match.any",
  contains: "playlist.subtitles.match.contains",
  extra: "playlist.subtitles.match.extra",
  exact: "playlist.subtitles.match.exact",
};

export interface FilenameMatchControlProps {
  /** Current strategy. */
  value: MatchMode;
  /** Strategy change. */
  onChange: (mode: MatchMode) => void;
  id?: string;
  className?: string;
}

export function FilenameMatchControl({
  value,
  onChange,
  id,
  className = "",
}: FilenameMatchControlProps): ReactElement {
  const t = useSubtitlesMsg();
  return (
    <Dropdown
      id={id}
      className={["lumen-plsubs-match", className].filter(Boolean).join(" ")}
      options={MATCH_MODES.map((mode) => ({ value: mode, labelKey: LABEL_KEY[mode] }))}
      value={value}
      onChange={(next) => onChange(next as MatchMode)}
      label={t("playlist.subtitles.match")}
      resolveLabel={t}
    />
  );
}
