// Card 4.F2 — MusicMode: layout re-weighting for `player.music` (doc 08
// §1.4 "Album art (music) 737×737 rx20, full-bleed blurred art behind";
// §1.6 `player.music`; §1.4 "Music-mode seek | waveform bars").
//
// The video canvas is de-emphasized/replaced by the album-art region. HONEST
// PLACEHOLDER: no artwork is fabricated — the engine does not expose artwork
// yet, so the art region renders a labelled placeholder square (no <img>, no
// gradients faking a cover). The waveform seek of the design needs
// engine-reported audio levels; until that data exists the standard C2
// seekbar is reused, restyled to the slim music-mode knob geometry via CSS,
// with an honest sr-only note. Playlist emphasis is layout-driven (the music
// layout's playlist region renders prominently — shell side).

import { useMsg } from "../../../i18n/messages";
import type { LayoutRegion } from "../layout";

export interface MusicModeProps {
  /** The album-art region (center anchor, from the music layout). */
  readonly artRegion: LayoutRegion;
}

export function MusicMode({ artRegion }: MusicModeProps) {
  const m = useMsg();
  return (
    <div
      data-region-id={artRegion.id}
      role="region"
      aria-label={m(artRegion.labelKey)}
      className="lumen-music__art"
      data-placeholder="artwork-engine-data-unwired"
    >
      {/* Honest placeholder: labelled square, no fabricated album art (the
          blurred full-bleed treatment arrives with engine artwork data). */}
      <p className="lumen-music__art-hint">{m("player.art.placeholder")}</p>
      {/* Waveform seek note (doc 08 §1.4 "Music-mode seek"): the design's
          waveform needs engine audio-level data — standard seek until then. */}
      <p className="lumen-player__sr-note">{m("player.music.waveform-placeholder")}</p>
    </div>
  );
}
