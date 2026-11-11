// w26.b11 — the matched-pairs review list ("Add Subtitles to Playlist/
// Folder.svg" + Folder-1 body): 15 rows pairing every media file with the
// subtitle file the current strategy (sub filename.svg) selected for it —
// media glyph + filename · dotted leader · subtitle glyph + filename.
//
// Frame anatomy (Folder.svg Frame 223 + "subtitle multiple" rows):
//   * 40px row pitch, transparent bands (this state has NO row fills —
//     selection lives in the Folder-2/3 candidate lists);
//   * media glyph 18×16 (stroke 1.5 #6B6B6B) at x33, media name #969696
//     at x68.7 (13–14px);
//   * dotted leader: Ø1 dots on an ~8.15px pitch, #373737 (→ nearest token
//     stroke-border #404040 — delta logged), vertically centered, spanning
//     the free space between the two names;
//   * subtitle glyph 18×20 at x549, subtitle name at x589.
// Rows arrive paired + natural-ordered from matching.pairSubtitles.

import type { ReactElement } from "react";
import { MediaFileGlyph, SubtitleFileGlyph } from "./SubtitleGlyphs";
import { useSubtitlesMsg } from "./useSubtitlesMsg";
import type { SubtitlePair } from "./matching";

export interface MatchedPairsListProps {
  /** Paired rows (pairSubtitles output — natural order by media name). */
  pairs: readonly SubtitlePair[];
  /** Accessible name override. */
  label?: string;
  id?: string;
}

export function MatchedPairsList({ pairs, label, id }: MatchedPairsListProps): ReactElement {
  const t = useSubtitlesMsg();
  if (pairs.length === 0) {
    return (
      <div className="lumen-plsubs-empty" id={id}>
        <p className="lumen-plsubs-empty__title">{t("playlist.subtitles.empty-title")}</p>
        <p className="lumen-plsubs-empty__body">{t("playlist.subtitles.empty-body")}</p>
      </div>
    );
  }
  return (
    <ul
      id={id}
      role="list"
      aria-label={label ?? t("playlist.subtitles.pairs-label")}
      className="lumen-plsubs-pairs"
    >
      {pairs.map((pair) => (
        <li key={pair.mediaName} className="lumen-plsubs-pair" role="listitem">
          <span className="lumen-plsubs-pair__icon" aria-hidden="true">
            <MediaFileGlyph size={19} />
          </span>
          <span className="lumen-plsubs-pair__name">{pair.mediaName}</span>
          {/* Dotted leader — the frame's Ø1-dot run between the two names
           * (Vector 77: 8.15px pitch); the dot ink is the hairline token. */}
          <span className="lumen-plsubs-pair__leader" aria-hidden="true" />
          {/* The subtitle side is a FIXED column (frame: icon x549 in the
           * 1008-wide body ≈ the subtitle one.svg 479px row width) — the
           * leader spans only the gap, it never pushes the column right. */}
          <span className="lumen-plsubs-pair__side">
            <span className="lumen-plsubs-pair__icon" aria-hidden="true">
              <SubtitleFileGlyph size={19} />
            </span>
            <span className="lumen-plsubs-pair__name">{pair.subtitleName}</span>
          </span>
        </li>
      ))}
    </ul>
  );
}
