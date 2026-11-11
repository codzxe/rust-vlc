// w26.c2 — ToolSliderRow: one dynamics-tool row, per the shared tool-card
// grammar (w26.a20 analysis §A4–A6/§D1, frames "Audio - Compressor
// Settings.svg" / "Audio - Spatializer Settings.svg" / "Audio - Stereo
// Widener.svg"):
//
//   label #6B6B6B ≈15px left · plain value #6B6B6B right (no chip — these
//   three frames are the PLAIN-value family, unlike the EQ's chip rows) ·
//   full-width slider 33px below the label: track 328×16 rx6 #090909, flat
//   #371F06 fill 12px rx4 inset 2, thumb 8×24 rx4 #AB6314, optional degree
//   ticks (6px marks, on the track centerline, drawn over the fill with the
//   thumb over its own tick — the frame z-order).
//
// The slider is the standard components/slider Slider (mission pin: not
// EQSlider); the tool geometry rides on the dynamics-slider class scope in
// dynamics.css. Fully controlled: local card state flows back in.

import type { ReactElement } from "react";
import { Slider } from "../../../../components/slider";
import { useDynamicsMsg } from "./copy";
import type { DynamicsRowDef } from "./types";

/** Thumb half-width in track-percent (8px thumb on a 328px track ≈ ±1.2%,
 * padded to 1.5%) — the tick under the thumb is covered, per the frames'
 * z-order (thumb paints last). */
const COVERED_TICK_BAND = 1.5;

export interface ToolSliderRowProps {
  /** Row definition (label, range, format, ticks). */
  readonly row: DynamicsRowDef;
  /** Current value (card-local state). */
  readonly value: number;
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
}

/** Renders one label/value/slider row (see module header for the grammar). */
export function ToolSliderRow({ row, value, onChange }: ToolSliderRowProps): ReactElement {
  const m = useDynamicsMsg();
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
