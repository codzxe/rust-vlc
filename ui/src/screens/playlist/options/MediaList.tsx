// w26.b10 — media (list) view (`media.svg`): 51px rows — 32×32 r2 media
// tile, title beside it, orange watched-progress bar under the tile, and a
// "•••" actions button that appears on hover/focus. The frame pins FOUR row
// states: watched (dim title + full bar), unwatched, hover-watched and
// hover-unwatched (bright title + •••).
//
// Watched state arrives through the metadata seam; with no seam wired every
// row renders honestly unwatched (no bar, normal title). One click on a row
// plays the item (v2 flow brief §1 single-click intent); the engine queue
// itself is never reordered by views.
//
// w26.f06 — ENGINE vs LOCAL scope: with an engine PlaylistProvider ancestor
// rows stay the frame's click-to-play rows. In the /playlist saved-playlist
// browse (local data, names only) there is NO play seam — the row renders
// as honest presentation, never a dead click target (the "no dead
// affordance" rule).

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import { VirtualList } from "../../../components/list";
import { usePlaylistOptional, type PlaylistEntry } from "../PlaylistProvider";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";
import { entryDisplayTitle, type PlaylistEntryMeta } from "./sorting";
import { useOptionsMsg } from "./useOptionsMsg";

/** media.svg row pitch: tiles at y 20 / 71 / 122 / 173 — 51px. Structural. */
export const MEDIA_ROW_HEIGHT = 51;

export interface MediaListProps {
  /** "•••" hover affordance callback (the playlist context-menu seam). */
  onEntryMenu?: (entry: PlaylistEntry) => void;
}

export function MediaList({ onEntryMenu }: MediaListProps): ReactElement {
  const m = useMsg();
  const t = useOptionsMsg();
  const playlist = usePlaylistOptional();
  const { sortedEntries, metaOf } = usePlaylistOptions();

  if (sortedEntries.length === 0) return <></>;
  const currentId = playlist?.currentId ?? null;
  return (
    <VirtualList
      data={sortedEntries}
      itemHeight={MEDIA_ROW_HEIGHT}
      label={t("playlist.options.list-label")}
      className="lumen-plo-media"
      scrollToIndex={
        currentId === null
          ? null
          : (() => {
              const index = sortedEntries.findIndex((entry) => entry.id === currentId);
              return index >= 0 ? { index, align: "start" } : null;
            })()
      }
      semantics="grid"
      renderItem={(entry) => (
        <MediaRow
          key={entry.id}
          isCurrent={entry.id === currentId}
          title={entryDisplayTitle(entry, m("playlist.entry.unknown-title"))}
          meta={metaOf(entry)}
          onPlay={playlist !== null ? () => playlist.playItem(entry.id) : undefined}
          onMenu={onEntryMenu ? () => onEntryMenu(entry) : undefined}
          menuLabel={t("playlist.options.entry-menu")}
        />
      )}
    />
  );
}

function MediaRow({
  isCurrent,
  title,
  meta,
  onPlay,
  onMenu,
  menuLabel,
}: {
  isCurrent: boolean;
  title: string;
  meta: PlaylistEntryMeta | undefined;
  /** Play seam — undefined in local scope (no dead click targets). */
  onPlay: (() => void) | undefined;
  onMenu: (() => void) | undefined;
  menuLabel: string;
}): ReactElement {
  const watched = meta?.watched === true;
  // Watched progress 0..1; a fully watched item reports watched=true with
  // no partial progress — the frame's completed bar (media.svg Rectangle 12).
  const progress = meta?.watchedProgress ?? (watched ? 1 : 0);
  return (
    <div
      className={[
        "lumen-plo-mrow",
        watched ? "lumen-plo-mrow--watched" : "",
        isCurrent ? "lumen-plo-mrow--current" : "",
        onPlay === undefined ? "lumen-plo-mrow--inert" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      data-watched={watched || undefined}
      aria-current={isCurrent || undefined}
      onClick={onPlay}
    >
      <span className="lumen-plo-mrow__tile" aria-hidden="true">
        {progress > 0 && (
          <span
            className="lumen-plo-mrow__progress"
            style={{ width: `${Math.round(Math.max(0, Math.min(1, progress)) * 100)}%` }}
          />
        )}
      </span>
      <span className="lumen-plo-mrow__title">{title}</span>
      {onMenu !== undefined && (
        <button
          type="button"
          className="lumen-plo-mrow__menu"
          aria-label={menuLabel}
          onClick={(event) => {
            event.stopPropagation();
            onMenu();
          }}
        >
          <span className="lumen-plo-mrow__dots" aria-hidden="true">
            <i />
            <i />
            <i />
          </span>
        </button>
      )}
    </div>
  );
}
