// w26.b10 — thumbnail (grid) view (`thumbnail.svg`): 272×189 r14 cards on a
// `#191919`-class panel — r4 artwork well (252×142, 10px inset), quiet
// centered h:mm:ss duration in the 37px footer. Cards are buttons: one
// click plays the item (v2 flow brief §1 single-click intent).
//
// The frame's artwork is a pattern placeholder — the engine has no artwork
// seam yet, so the well renders honestly empty with a quiet thumbnail glyph.

import type { ReactElement } from "react";
import { Icon } from "../../../components/icon";
import { useMsg } from "../../../i18n/messages";
import { usePlaylist } from "../PlaylistProvider";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";
import { entryDisplayTitle, formatDurationClock } from "./sorting";
import { useOptionsMsg } from "./useOptionsMsg";

export function ThumbnailGrid(): ReactElement {
  const m = useMsg();
  const t = useOptionsMsg();
  const playlist = usePlaylist();
  const { sortedEntries } = usePlaylistOptions();

  if (sortedEntries.length === 0) return <></>;
  return (
    <ul className="lumen-plo-grid" aria-label={t("playlist.options.grid-label")}>
      {sortedEntries.map((entry) => {
        const duration = formatDurationClock(entry.durationSeconds);
        return (
          <li key={entry.id} className="lumen-plo-grid__cell">
            <button
              type="button"
              className="lumen-plo-card"
              onClick={() => playlist.playItem(entry.id)}
              title={entryDisplayTitle(entry, m("playlist.entry.unknown-title"))}
            >
              <span className="lumen-plo-card__art" aria-hidden="true">
                <Icon name="thumbnail" />
              </span>
              <span className="lumen-plo-card__duration">
                {duration ?? m("playlist.entry.unknown-duration")}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
