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

import { useId, useMemo, useRef, useState } from "react";
import type { ReactElement } from "react";
import { Button, IconButton } from "../../../components/button";
import { Icon } from "../../../components/icon";
import { Dropdown } from "../../../components/menu";
import { Dialog } from "../../../components/surface";
import type { SurfaceCloseReason } from "../../../components/surface";
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
  const windowRef = useRef<HTMLDivElement | null>(null);
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

  // w26.gfix-E (g19 P1-2, scoped seam): the Dialog focus-trap listens for
  // Escape with a NATIVE keydown listener on the dialog node — a bubble-phase
  // ancestor of React's delegated root — so it fires BEFORE the shared
  // Dropdown's React handler (the Lead's stopPropagation fix never runs
  // inside a Dialog) and one Escape closed the dropdown AND the whole dialog.
  // Guard: while any dropdown list is open in this window, an escape-dismiss
  // is redirected to that dropdown (its trigger click IS the Dropdown's own
  // close-and-restore path — list closes, focus returns to the trigger); the
  // dialog closes only once no list is open. Root cause lives in the shared
  // surface/menu primitives (logged for the Lead); this seam is scoped here.
  const guardedClose = (reason: SurfaceCloseReason): void => {
    const openList = windowRef.current?.querySelector(".lumen-dropdown__list") ?? null;
    if (openList !== null) {
      openList
        .closest(".lumen-dropdown")
        ?.querySelector<HTMLButtonElement>(".lumen-dropdown__trigger")
        ?.click();
      return;
    }
    void reason; // the host seam is reason-less (matches AddSubtitlesDialogProps)
    onClose();
  };

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
    <Dialog open={open} onClose={guardedClose} labelledBy={titleId} closeOnOutsideClick className="lumen-plsubs">
      <div
        className="lumen-plsubs__window"
        ref={(node) => {
          windowRef.current = node;
        }}
      >
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
                 * path segments, #151515 h34 r6, slashes quieter. w26.gfix-E
                 * (g19 P2-1): the pills row carries ONLY the frame's three
                 * controls (200 + 200 + flex crumb = 1030px, exactly the
                 * frame's budget) — the match dropdown moved to the footer. */}
                <div className="lumen-plsubs__crumb" aria-label={t("playlist.subtitles.folder-path")}>
                  <Icon name="folder" size={18} />
                  <span className="lumen-plsubs__crumb-path">
                    {folderPath.length === 0 ? null : folderPath.join(" / ")}
                  </span>
                </div>
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

        {/* Footer — frame Frame 140/141: Cancel #1B1B1B left, Save (tinted)
         * right, 100×32 r6 at 20px insets; Save is disabled until the flow
         * has something to commit (b09 dialog precedent). w26.gfix-E (g19
         * P2-1): the filename-matching dropdown (sub filename.svg — a
         * control no Folder frame pins) rides the footer's open middle
         * span, between Cancel and Save. The header band stays the frame's
         * exact three-control row and the 15-pair review keeps its static
         * y118–718 budget (a body filter row would push rows 14–15 under
         * the fold — the very defect this fixes). It only mounts in the
         * pairs mode it drives; candidate/online states match their frames'
         * two-button footers. */}
        <footer className="lumen-plsubs__foot">
          <Button className="lumen-plsubs__cancel" onClick={onClose}>
            {t("playlist.subtitles.cancel")}
          </Button>
          {pairsMode && <FilenameMatchControl value={matchMode} onChange={setMatchMode} />}
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
