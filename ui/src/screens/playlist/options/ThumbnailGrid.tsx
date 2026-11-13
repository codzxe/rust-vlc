// w26.b10 — thumbnail (grid) view (`thumbnail.svg`): 272×189 r14 cards on a
// `#191919`-class panel — r4 artwork well (252×142, 10px inset), quiet
// centered h:mm:ss duration in the 37px footer. Cards are buttons: one
// click plays the item (v2 flow brief §1 single-click intent).
//
// The frame's artwork is a pattern placeholder — the engine has no artwork
// seam yet, so the well renders honestly empty with a quiet thumbnail glyph.
//
// w26.f06 — ENGINE vs LOCAL scope: with an engine PlaylistProvider ancestor
// (player dock / harness) cards stay the frame's play buttons. In the
// /playlist saved-playlist browse (local data, names only) there is NO play
// seam — the card renders as honest presentation, never a dead button (the
// "no dead affordance" rule, MediaList's ••• precedent).

import type { ReactElement } from "react";
import { Icon } from "../../../components/icon";
import { useMsg } from "../../../i18n/messages";
import { usePlaylistOptional } from "../PlaylistProvider";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";
import { entryDisplayTitle, formatDurationClock } from "./sorting";
import { useOptionsMsg } from "./useOptionsMsg";

export function ThumbnailGrid(): ReactElement {
  const m = useMsg();
  const t = useOptionsMsg();
  const playlist = usePlaylistOptional();
  const { sortedEntries } = usePlaylistOptions();

  if (sortedEntries.length === 0) return <></>;
  return (
    <ul className="lumen-plo-grid" aria-label={t("playlist.options.grid-label")}>
      {sortedEntries.map((entry) => {
        const duration = formatDurationClock(entry.durationSeconds);
        const title = entryDisplayTitle(entry, m("playlist.entry.unknown-title"));
        const cardBody = (
          <>
            <span className="lumen-plo-card__art" aria-hidden="true">
              <Icon name="thumbnail" />
            </span>
            <span className="lumen-plo-card__duration">
              {duration ?? m("playlist.entry.unknown-duration")}
            </span>
          </>
        );
        return (
          <li key={entry.id} className="lumen-plo-grid__cell">
            {playlist !== null ? (
              <button
                type="button"
                className="lumen-plo-card"
                onClick={() => playlist.playItem(entry.id)}
                /* w26.k15 (j15 P3-4): name the card by its TITLE — name-from-
                 * content made every card announce its duration text
                 * ("00:02:10, button"); the visible duration stays a child,
                 * `title` keeps the pointer tooltip. */
                aria-label={title}
                title={title}
              >
                {cardBody}
              </button>
            ) : (
              /* Local scope (w26.f06): presentation card — role="img" + the
               * title gives the inert card an accessible name (a bare div's
               * aria-label is ignored); it names the media, not the duration. */
              <div className="lumen-plo-card lumen-plo-card--inert" role="img" aria-label={title} title={title}>
                {cardBody}
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}
