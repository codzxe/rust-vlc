// Card 4.F2 — MusicMode: layout re-weighting for `player.music`, composed
// to the W26 frames `design/figma/Player - Music.svg` + `Player - Music-1.svg`
// — the ONE theatrical screen of the redesign:
//   * large centered artwork treatment (frame: 738×737 rx20 art on the
//     1920×1080 stage) — an honest placeholder face until the engine
//     reports artwork (no fabricated art, no Solstice sun),
//   * ambient tint — a quiet accent-tinted radial glow behind the art (the
//     frame's full-bleed art backdrop), staying inside the near-black +
//     orange system (NO sunset gradients),
//   * quiet meta — the engine-reported title under the art (bright), or the
//     honest placeholder caption when nothing is playing,
//   * compact controls — the real C2 ControlBar drives transport in the
//     controls-bar region (PlayerScreen); this component owns the stage.
//
// The waveform-tick strip (frame bottom bar: 2px #404040 ticks, played
// portion orange) is a STATIC decorative silhouette — engine audio levels
// do not exist yet, so bar heights are not audio data (honesty rule doc 05
// §1 C4; the sr-only note says so). The PLAYED tint is real: it clips to
// the engine-reported position/duration. The standard C2 seekbar still
// drives transport.

import { useMsg } from "../../../i18n/messages";
import { usePlayerControls } from "../context";
import { usePlaylist } from "../../playlist";
import { Icon } from "../../../components";
import type { CSSProperties } from "react";
import type { LayoutRegion } from "../layout";

export interface MusicModeProps {
  /** The album-art region (center anchor, from the music layout). */
  readonly artRegion: LayoutRegion;
}

/**
 * Decorative waveform silhouette heights (percent). Static by design: a
 * moving or engine-claimed waveform would imply audio-level data that does
 * not exist yet (honesty rule, doc 05 §1 C4).
 */
const WAVEFORM_BARS: readonly number[] = [
  38, 62, 84, 55, 96, 70, 44, 78, 58, 90, 66, 40, 82, 52, 72, 46, 88, 60, 76,
  50, 92, 56, 68, 42, 74, 64, 86, 48, 80, 54, 94, 58, 76, 44, 90, 62, 72, 50,
  82, 66, 56, 88, 40, 78, 68, 92, 52, 84, 60,
];

export function MusicMode({ artRegion }: MusicModeProps) {
  const m = useMsg();
  const { state } = usePlayerControls();
  const playlist = usePlaylist();

  // Real data only (never a fabricated title): engine truth first, the
  // current playlist entry's title second.
  const entryTitle =
    playlist.entries.find((entry) => entry.id === playlist.currentId)?.title ?? null;
  const title = state.mediaTitle ?? entryTitle;

  // Played fraction of the tick strip — engine truth (position/duration).
  const duration = state.durationSeconds ?? 0;
  const progress = duration > 0 ? Math.min(Math.max(state.position / duration, 0), 1) : 0;

  return (
    <div
      data-region-id={artRegion.id}
      role="region"
      aria-label={m(artRegion.labelKey)}
      className="lumen-music__art"
      data-placeholder="artwork-engine-data-unwired"
    >
      <div className="lumen-music__stage">
        {/* Honest placeholder face: no fabricated album art (real artwork
            renders here once the engine reports it). The ambient tint is
            the frame's quiet accent glow — purely decorative. */}
        <figure className="lumen-music__artwork" aria-hidden="true">
          <span className="lumen-music__artwork-glyph">
            <Icon name="music" size={64} variant="strong" />
          </span>
        </figure>

        {/* Quiet meta — the title/artist layout slot. Engine truth while a
            session is up; the honest placeholder caption otherwise (the
            same slot renders real metadata when the engine reports it). */}
        {title === null ? (
          <p className="lumen-music__caption">{m("player.art.placeholder")}</p>
        ) : (
          <h2 className="lumen-music__title">{title}</h2>
        )}
      </div>

      {/* Waveform tick strip (frame: 2px ticks, unplayed #404040, played
          orange). Static silhouette + engine-truth played clip; decorative
          — the honest note below carries the meaning. */}
      <div
        className="lumen-music__waveform"
        aria-hidden="true"
        style={{ "--lumen-wave-progress": `${progress * 100}%` } as CSSProperties}
      >
        <div className="lumen-music__waveform-row">
          {WAVEFORM_BARS.map((height, index) => (
            <span key={index} style={{ height: `${height}%` }} />
          ))}
        </div>
        <div className="lumen-music__waveform-row lumen-music__waveform-row--played">
          {WAVEFORM_BARS.map((height, index) => (
            <span key={index} style={{ height: `${height}%` }} />
          ))}
        </div>
      </div>
      <p className="lumen-player__sr-note">{m("player.music.waveform-placeholder")}</p>
    </div>
  );
}
