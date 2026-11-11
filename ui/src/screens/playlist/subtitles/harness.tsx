// w26.b11 — DEV-ONLY vision harness (never imported by app routes): mounts
// the subtitle-flow frame states on the app surface so the comparison loop
// (contract §5) can screenshot them against design/figma-renders/
// {subtitle_one,subtitle_multiple,sub_filename}.png and the "Add Subtitles
// to Playlist/Folder*" states (rendered from the SVGs — no PNG renders
// exist; cairosvg exports live in the lane log record).
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly — see the lane log for the loop record.
//
//   W26B11_HARNESS_OUT=/tmp/w26b11-harness npm run test -- harness.test

import type { ReactElement } from "react";
import { AddSubtitlesDialog } from "./AddSubtitlesDialog";
import { FilenameMatchControl } from "./FilenameMatchControl";
import { SubtitlePickList } from "./SubtitlePickList";
import { SubtitlePickRow } from "./SubtitlePickRow";
import { DEFAULT_MATCH_MODE } from "./matching";
import "./harness.css";

/** Folder.svg fixtures: 15 episodes with their matching .srt files. */
export const HARNESS_EPISODES: readonly string[] = Array.from(
  { length: 15 },
  (_, index) => `The_Wand_S01E${String(index + 1).padStart(2, "0")}.mp4`,
);

/** Folder.svg candidates: the 15 matching subtitle files. */
export const HARNESS_FOLDER_CANDIDATES: readonly string[] = HARNESS_EPISODES.map(
  (name) => name.replace(/\.mp4$/, ".srt"),
);

/** Folder-2 candidates: six language variants of episode 1. */
export const HARNESS_MULTI_CANDIDATES: readonly string[] = [
  "The_Wand_S01E01_English.srt",
  "The_Wand_S01E01_Spanish.srt",
  "The_Wand_S01E01_Japanese.srt",
  "The_Wand_S01E01_German.srt",
  "The_Wand_S01E01_Dutch.srt",
  "The_Wand_S01E01_French.srt",
];

/** Folder-3 candidates: the online search results. */
export const HARNESS_ONLINE_CANDIDATES: readonly string[] = [
  "The_Wand_S01E01_English.srt",
  "The_Wand_S01E01_EN.srt",
];

export const HARNESS_PATH: readonly string[] = ["Videos", "Shows", "The Wand", "S01"];

/** State 1/2 — the matched-pairs review (Folder.svg / Folder-1.svg). */
export function FolderPairsHarness(): ReactElement {
  return (
    <AddSubtitlesDialog
      open
      onClose={() => {}}
      mediaNames={HARNESS_EPISODES}
      candidates={HARNESS_FOLDER_CANDIDATES}
      folderPath={HARNESS_PATH}
    />
  );
}

/** State 3 — the multi-subtitle pick (Folder-2.svg; select rows 1–2 in the
 * driving test — the frame's two checked bands). */
export function FolderMultiHarness(): ReactElement {
  return (
    <AddSubtitlesDialog
      open
      onClose={() => {}}
      initialDestination="file"
      mediaNames={["The_Wand_S01E01.mp4"]}
      candidates={HARNESS_MULTI_CANDIDATES}
      folderPath={HARNESS_PATH}
    />
  );
}

/** State 4 — the online pick (Folder-3.svg; select row 1 in the test). */
export function FolderOnlineHarness(): ReactElement {
  return (
    <AddSubtitlesDialog
      open
      onClose={() => {}}
      initialDestination="file"
      initialSource="online"
      mediaNames={["The_Wand_S01E01.mp4"]}
      candidates={HARNESS_ONLINE_CANDIDATES}
    />
  );
}

/** sub filename.svg closeup — the strategy dropdown on a fragment stage:
 * closed state + open state stacked like the frame (the test opens the
 * SECOND instance before serializing). */
export function SubFilenameHarness(): ReactElement {
  return (
    <div className="w26b11-stage w26b11-stage--filename">
      <div className="w26b11-cell">
        {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
        <p className="w26b11-caption">sub filename — state=default</p>
        <div className="w26b11-frame w26b11-frame--filename">
          <FilenameMatchControl value={DEFAULT_MATCH_MODE} onChange={() => {}} />
        </div>
        {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
        <p className="w26b11-caption">sub filename — state=open</p>
        <div className="w26b11-frame w26b11-frame--filename">
          <FilenameMatchControl value={DEFAULT_MATCH_MODE} onChange={() => {}} />
        </div>
      </div>
    </div>
  );
}

/** subtitle one/multiple.svg closeups — the row fragments at the frames'
 * 519px width (default + hover-arrows variant, selected band + check). */
export function SubtitleRowsHarness(): ReactElement {
  return (
    <div className="w26b11-stage w26b11-stage--rows">
      <div className="w26b11-cell">
        {/* eslint-disable-next-line local/i18n-no-literals -- dev-only harness caption: the figma frame name itself, not product copy */}
        <p className="w26b11-caption">subtitle one / subtitle multiple</p>
        <div className="w26b11-frame w26b11-frame--rows">
          <SubtitlePickRow id="w26b11-row-default" fileName="The_Wand_S01E01.srt" selected={false} />
          <SubtitlePickRow id="w26b11-row-multiple" fileName="The_Wand_S01E01.srt" selected={false} multiple />
          <SubtitlePickRow id="w26b11-row-selected" fileName="The_Wand_S01E01.srt" selected />
        </div>
        <div className="w26b11-frame w26b11-frame--rows">
          <SubtitlePickList
            candidates={HARNESS_MULTI_CANDIDATES}
            selected={["The_Wand_S01E01_English.srt", "The_Wand_S01E01_Spanish.srt"]}
            onSelectedChange={() => {}}
            multiple
          />
        </div>
      </div>
    </div>
  );
}
