// w26.c4 — VideoSliderRow: one video-tool slider row. A 1:1 mirror of the
// dynamics lane's ToolSliderRow (w26.c2) — same DOM, same .dynamics-* classes,
// same tick overlay — with the label resolved through THIS lane's useVideoMsg
// seam: the dynamics row's resolver + label-key union are hard-wired to the
// w26.c2 keys, so w26.c4 label keys (Similarity, Sepia Intensity, Sharpen
// Sigma…) cannot resolve through it. Same reason the equalizer lane ships
// useEqMsg + EqBandRow (w26.c1); the dynamics module is never edited.
//
// Grammar (w26.a20 analysis §B1/§B3/§D1, frames "Video - Colors.svg" /
// "Video - Essential.svg"): label #6B6B6B ≈15px left, NO value column (both
// frames' slider rows are label-only — video.css hides the shared value
// span; the slider's aria-valuetext still reports every value), full-width
// slider below: track 328×16 rx6 #090909, flat #371F06 fill 12px rx4 inset 2,
// thumb 8×24 rx4 #AB6314, optional degree ticks (6px marks on the track
// centerline, drawn over the fill with the thumb over its own tick — the
// frame z-order).

import type { ReactElement } from "react";
import { Slider } from "../../../../components/slider";
import { useVideoMsg } from "./copy";
import type { VideoSliderRowDef } from "./types";

/** Thumb half-width in track-percent (8px thumb on a 328px track ≈ ±1.2%,
 * padded to 1.5%) — the tick under the thumb is covered, per the frames'
 * z-order (thumb paints last). */
const COVERED_TICK_BAND = 1.5;

export interface VideoSliderRowProps {
  /** Row definition (label, range, format, ticks). */
  readonly row: VideoSliderRowDef;
  /** Current value (card-local state). */
  readonly value: number;
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
}

/** Renders one label/slider row (see module header for the grammar). */
export function VideoSliderRow({
  row,
  value,
  onChange,
}: VideoSliderRowProps): ReactElement {
  const m = useVideoMsg();
  const label = m(row.labelKey);
  const range = row.max - row.min;
  const percent = range > 0 ? ((value - row.min) / range) * 100 : 0;
  const ticks = row.ticks ?? 0;

  return (
    <div className="dynamics-row" data-row={row.id}>
      <div className="dynamics-row__head">
        <span className="dynamics-row__label">{label}</span>
        <span className="dynamics-row__value">{row.format(value)}</span>
      </div>
      <div className="dynamics-row__slider">
        <Slider
          className="dynamics-slider"
          value={value}
          min={row.min}
          max={row.max}
          step={row.step}
          onChangeLive={onChange}
          onCommit={onChange}
          formatValue={row.format}
          aria-label={label}
        />
        {ticks > 0 ? (
          <div className="dynamics-slider__ticks" aria-hidden="true">
            {Array.from({ length: ticks }, (_, index) => {
              // Frame degree marks: evenly stepped, one step in from each
              // end (i/(n+1)) — the default value sits exactly on a tick.
              const left = ((index + 1) / (ticks + 1)) * 100;
              const covered = Math.abs(left - percent) <= COVERED_TICK_BAND;
              return (
                <span
                  key={index}
                  className={
                    covered
                      ? "dynamics-slider__tick dynamics-slider__tick--covered"
                      : "dynamics-slider__tick"
                  }
                  style={{ left: `${left.toFixed(3)}%` }}
                />
              );
            })}
          </div>
        ) : null}
      </div>
    </div>
  );
}
