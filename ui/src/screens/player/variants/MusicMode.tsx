// Card 4.F2 — MusicMode: layout re-weighting for `player.music` (doc 08
// §1.4 "Album art (music) 737×737 rx20, full-bleed blurred art behind";
// §1.6 `player.music`; §1.4 "Music-mode seek | waveform bars").
//
// The video canvas is de-emphasized/replaced by the album-art region. HONEST
// PLACEHOLDER: no artwork is fabricated — the engine does not expose artwork
// yet, so the art region renders the Solstice sun disc (glowing, the visual
// stand-in for the album face) above the labelled placeholder caption. The
// caption block keeps the title/artist LAYOUT slot: when engine metadata
// lands, the same block renders the real title/artist instead. The waveform
// seek of the design needs engine-reported audio levels; until that data
// exists the placeholder renders as a STATIC bar row in the sunset
// visualizer gradient (animated bars would fake audio activity — doc 05 §1
// C4), with the standard C2 seekbar still driving transport and an honest
// sr-only note. Playlist emphasis is layout-driven (the music layout's
// playlist region renders prominently — shell side).

import { useMsg } from "../../../i18n/messages";
import type { LayoutRegion } from "../layout";

export interface MusicModeProps {
  /** The album-art region (center anchor, from the music layout). */
  readonly artRegion: LayoutRegion;
}

/**
 * Decorative waveform silhouette heights (percent). Static by design: a
 * moving waveform would imply engine-reported audio levels that do not
 * exist yet (honesty rule, doc 05 §1 C4).
 */
const WAVEFORM_BARS: readonly number[] = [
  38, 62, 84, 55, 96, 70, 44, 78, 58, 90, 66, 40, 82, 52, 72, 46, 88, 60, 76,
  50, 92, 56, 68, 42,
];

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
      <figure className="lumen-music__stage">
        {/* Honest placeholder disc: no fabricated album art (the blurred
            full-bleed treatment arrives with engine artwork data). The glow
            pulse is the sanctioned solstice-glow (reduced-motion safe). */}
        <span className="solstice-sun solstice-glow lumen-music__sun" aria-hidden="true" />
        {/* Title/artist layout slot — the honest placeholder caption until
            the engine reports real metadata. */}
        <figcaption className="lumen-music__caption">
          {m("player.art.placeholder")}
        </figcaption>
      </figure>

      {/* Waveform seek placeholder (doc 08 §1.4 "Music-mode seek"): sunset
          visualizer gradient tokens, static until audio-level data exists.
          Decorative — the honest note below carries the meaning. */}
      <div className="lumen-music__waveform" aria-hidden="true">
        {WAVEFORM_BARS.map((height, index) => (
          <span key={index} style={{ height: `${height}%` }} />
        ))}
      </div>
      <p className="lumen-player__sr-note">{m("player.music.waveform-placeholder")}</p>
    </div>
  );
}
