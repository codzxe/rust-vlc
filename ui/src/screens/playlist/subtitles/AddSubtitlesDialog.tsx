// w26.b11 — the Add-Subtitles flow (playlist subtitle dialogs).
//
// Frames (7): the four full-window "Add Subtitles to Playlist/Folder"
// states + the two candidate-row fragments + the filename-matching
// dropdown:
//   * Folder.svg / Folder-1.svg — the matched-pairs review ("To This
//     Folder" + "From Computer": 15 rows of media ↔ subtitle pairs);
//   * Folder-2.svg — the multi-subtitle pick ("To This File" + "From
//     Computer": 6 candidate rows, first two selected — subtitle_multiple);
//   * Folder-3.svg — the online pick ("To This File" + "From OpenSubtitles"
//     + search field + language pill: 2 candidates, first selected —
//     subtitle_one);
//   * sub filename.svg — the matching strategy dropdown that drives the
//     pairing (Off / Any title / Contains title / Title with extra
//     characters / Exact title match).
//
// Anatomy (measured on Folder.svg, 1048×800): full-window rx14 overlay
// #0E0E0E @95% · #090909 header band 0–96px (title "Add Subtitles"
// #8F8F8F + ✕ at 20px insets; pills h34 r6 #151515 with #969696 labels +
// ∨ chevrons) · body rows on the overlay · footer Cancel 100×32 r6 #1B1B1B
// left + Save 100×32 r6 #371F06 (tinted) right at 20px insets.
//
// HONESTY: the engine catalog has no subtitle-attach command and no
// folder-scan / OpenSubtitles service (doc 05 §2 gap) — media names and
// candidates arrive through props (the host wires the seams), Save reports
// the selection through `onSave` and never fakes persistence. Mount recipe
// (Lead): inside the /playlist tree, `<AddSubtitlesDialog open={…}
// onClose={…} mediaNames={playlist.entries.map(entryDisplayTitle)}
// candidates={…} onSave={…} />` — see docs/waves/w26/logs/w26.b11.md.

import { useId, useMemo, useState } from "react";
import type { ReactElement } from "react";
import { Button, IconButton } from "../../../components/button";
import { Icon } from "../../../components/icon";
import { Dropdown } from "../../../components/menu";
import { Dialog } from "../../../components/surface";
import { naturalSort } from "../../../components/list";
import {
  DEFAULT_MATCH_MODE,
  pairSubtitles,
  type MatchMode,
  type SubtitlePair,
} from "./matching";
import { FilenameMatchControl } from "./FilenameMatchControl";
import { MatchedPairsList } from "./MatchedPairsList";
import { SubtitlePickList } from "./SubtitlePickList";
import { useSubtitlesMsg } from "./useSubtitlesMsg";
import "./subtitles.css";

/** Where the subtitles apply (Folder.svg "To This Folder" pill family). */
export type SubtitleDestination = "playlist" | "folder" | "file";

/** Where the subtitles come from ("From Computer" / "From OpenSubtitles"). */
export type SubtitleSource = "computer" | "online";

/** What Save commits — the host applies it (no engine command exists). */
export interface AddSubtitlesResult {
  readonly destination: SubtitleDestination;
  readonly source: SubtitleSource;
  readonly matchMode: MatchMode;
  /** Candidate modes: the picked files (natural order). */
  readonly selected: readonly string[];
  /** Review mode: the matched pairs (natural order by media name). */
  readonly pairs: readonly SubtitlePair[];
}

export interface AddSubtitlesDialogProps {
  open: boolean;
  onClose: () => void;
  /** Media file names in scope (the review list pairs them). */
  mediaNames?: readonly string[];
  /** Candidate subtitle files (folder scan or search results — host seam). */
  candidates?: readonly string[];
  /** Save commit hook (see HONESTY above). */
  onSave?: (result: AddSubtitlesResult) => void;
  /** Initial destination/source/match-mode (tests, harness, deep links). */
  initialDestination?: SubtitleDestination;
  initialSource?: SubtitleSource;
  initialMatchMode?: MatchMode;
  /** Breadcrumb path segments (Folder.svg "Videos / Shows / The Wand / S01"). */
  folderPath?: readonly string[];
}

const DESTINATIONS: readonly SubtitleDestination[] = ["playlist", "folder", "file"];
const SOURCES: readonly SubtitleSource[] = ["computer", "online"];

export function AddSubtitlesDialog({
  open,
  onClose,
  mediaNames = [],
  candidates = [],
  onSave,
  initialDestination = "folder",
  initialSource = "computer",
  initialMatchMode = DEFAULT_MATCH_MODE,
  folderPath = [],
}: AddSubtitlesDialogProps): ReactElement | null {
  const t = useSubtitlesMsg();
  const titleId = useId();
  const [destination, setDestination] = useState<SubtitleDestination>(initialDestination);
  const [source, setSource] = useState<SubtitleSource>(initialSource);
  const [matchMode, setMatchMode] = useState<MatchMode>(initialMatchMode);
  const [selected, setSelected] = useState<readonly string[]>([]);
  const [search, setSearch] = useState("");

  // The body mode per the frames: review pairs for folder/playlist-wide
  // computer adds (Folder.svg), multi-pick for one file from the computer
  // (Folder-2), single-pick for the online source (Folder-3).
  const pairsMode = source === "computer" && destination !== "file";

  // Hooks stay unconditional (rules-of-hooks); the dialog itself returns
  // null below when closed.
  const pairs = useMemo(
    () => (pairsMode ? pairSubtitles(mediaNames, candidates, matchMode) : []),
    [pairsMode, mediaNames, candidates, matchMode],
  );

  // Online search filters the candidates (honest contains, case-insensitive).
  const visibleCandidates = useMemo(() => {
    if (source === "computer") return candidates;
    const query = search.trim().toLowerCase();
    if (query === "") return candidates;
    return candidates.filter((name) => name.toLowerCase().includes(query));
  }, [source, candidates, search]);

  if (!open) return null;

  const save = (): void => {
    // Commit in NATURAL order regardless of the click order (the v2 flow
    // brief's natural-sort fix extends to the committed selection).
    onSave?.({
      destination,
      source,
      matchMode,
      selected: naturalSort(selected, (name) => name),
      pairs,
    });
    onClose();
  };

  return (
    <Dialog open={open} onClose={onClose} labelledBy={titleId} closeOnOutsideClick className="lumen-plsubs">
      <div className="lumen-plsubs__window">
        {/* Header band — frame: #090909 path 0–96px with the panel's r14
         * top corners; title row then the control pills. */}
        <header className="lumen-plsubs__head">
          <div className="lumen-plsubs__titlebar">
            <h2 id={titleId} className="lumen-plsubs__title">
              {t("playlist.subtitles.title")}
            </h2>
            <IconButton
              className="lumen-plsubs__close"
              label={t("playlist.subtitles.close")}
              size="sm"
              onClick={onClose}
            >
              <Icon name="close" size={16} />
            </IconButton>
          </div>
          <div className="lumen-plsubs__controls">
            <Dropdown
              className="lumen-plsubs__pill"
              options={DESTINATIONS.map((value) => ({
                value,
                labelKey: `playlist.subtitles.destination.${value}` as const,
              }))}
              value={destination}
              onChange={(next) => setDestination(next as SubtitleDestination)}
              label={t("playlist.subtitles.destination")}
              resolveLabel={t}
            />
            <Dropdown
              className="lumen-plsubs__pill"
              options={SOURCES.map((value) => ({
                value,
                labelKey: `playlist.subtitles.source.${value}` as const,
              }))}
              value={source}
              onChange={(next) => setSource(next as SubtitleSource)}
              label={t("playlist.subtitles.source")}
              resolveLabel={t}
            />
            {source === "computer" ? (
              <>
                {/* Breadcrumb pill — frame Frame 122: folder glyph + the
                 * path segments, #151515 h34 r6, slashes quieter. */}
                <div className="lumen-plsubs__crumb" aria-label={t("playlist.subtitles.folder-path")}>
                  <Icon name="folder" size={18} />
                  <span className="lumen-plsubs__crumb-path">
                    {folderPath.length === 0 ? null : folderPath.join(" / ")}
                  </span>
                </div>
                <FilenameMatchControl value={matchMode} onChange={setMatchMode} />
              </>
            ) : (
              <>
                {/* Search field — frame Folder-3: magnifier + query, same
                 * pill grammar as the breadcrumb. */}
                <div className="lumen-plsubs__search">
                  <Icon name="search" size={16} />
                  <input
                    type="text"
                    className="lumen-plsubs__search-input"
                    aria-label={t("playlist.subtitles.search")}
                    placeholder={t("playlist.subtitles.search.placeholder")}
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                  />
                </div>
                <Dropdown
                  className="lumen-plsubs__pill"
                  options={[{ value: "english", labelKey: "playlist.subtitles.language.english" as const }]}
                  value="english"
                  onChange={() => {
                    /* single language pinned by Folder-3 */
                  }}
                  label={t("playlist.subtitles.language")}
                  resolveLabel={t}
                />
              </>
            )}
          </div>
        </header>

        {/* Body — the review list or the candidate picker. */}
        <div className="lumen-plsubs__body">
          {pairsMode ? (
            <MatchedPairsList pairs={pairs} />
          ) : (
            <SubtitlePickList
              candidates={visibleCandidates}
              selected={selected}
              onSelectedChange={setSelected}
              multiple={source === "computer"}
            />
          )}
        </div>

        {/* Footer — frame Frame 140/141: Cancel #1B1B1B left, Save
         * (tinted) right, 100×32 r6 at 20px insets; Save is disabled until
         * the flow has something to commit (b09 dialog precedent). */}
        <footer className="lumen-plsubs__foot">
          <Button onClick={onClose}>{t("playlist.subtitles.cancel")}</Button>
          <Button
            variant="tinted"
            disabled={pairsMode ? pairs.length === 0 : selected.length === 0}
            onClick={save}
          >
            {t("playlist.subtitles.save")}
          </Button>
        </footer>
      </div>
    </Dialog>
  );
}
