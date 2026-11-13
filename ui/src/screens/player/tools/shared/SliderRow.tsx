// w26.k10 (j10 P2-3/P3-5; j11 P2-1/P3-5) — THE shared tool slider row.
// ToolSliderRow (dynamics) and VideoSliderRow (video) were 1:1 copies that
// had ALREADY drifted (video kept the pre-i07 full-width tick percent — the
// 1.7px paint error dynamics fixed); this module is the ONE row both lanes
// compose, extracted with the corrected w26.i07 paint domain.
//
// Grammar (w26.c2, frames "Audio - Compressor Settings.svg" / "Audio -
// Spatializer Settings.svg" / "Audio - Stereo Widener.svg"; the video frames
// ride the same row grammar — w26.a20 analysis §A4–A6/§B1/§B3/§D1):
//
//   label #6B6B6B ≈15px left · plain value #6B6B6B right (the PLAIN-value
//   family; the EQ's chip rows are a different grammar) · full-width slider
//   33px below the label: track 328×16 rx6 #090909, flat #371F06 fill 12px
//   rx4 inset 2, thumb 8×24 rx4 #AB6314, optional degree ticks (6px marks,
//   on the track centerline, drawn over the fill with the thumb over its own
//   tick — the frame z-order).
//
// The `.dynamics-*` classes + dynamics.css are the SHARED row grammar both
// lanes ride (video.css composes onto them verbatim — j11 ruling) — the
// class names are the cross-lane contract, not dynamics property.
//
// The slider is the standard components/slider Slider (mission pin: not
// EQSlider); the tool geometry rides on the dynamics-slider class scope in
// dynamics.css. Fully controlled: local card state flows back in.
//
// Generic over the label key: each lane passes its own i18n seam via
// `resolveLabel` (useDynamicsMsg / useVideoMsg) — the hard-wired key union
// that forced the w26.c4 copy is gone. `React.memo` (j10 P3-5): with a
// stable row def + value + sink, a drag on one row no longer re-renders the
// other rows of the card.

import { memo } from "react";
import type { ReactElement } from "react";
import { Slider } from "../../../../components/slider";

/** Thumb half-width in track-percent (8px thumb on a 328px track ≈ ±1.2%,
 * padded to 1.5%) — the tick under the thumb is covered, per the frames'
 * z-order (thumb paints last). */
const COVERED_TICK_BAND = 1.5;

/** One tool row's definition (label, range, format, ticks). */
export interface SliderRowDef<TKey extends string> {
  /** Stable row id (state key + data-row marker). */
  readonly id: string;
  /** Row label key (frame label column). */
  readonly labelKey: TKey;
  /** Display range minimum. */
  readonly min: number;
  /** Display range maximum. */
  readonly max: number;
  /** Commit step (keyboard arrows, pointer snap). */
  readonly step: number;
  /** Value-column string + slider aria-valuetext (frame format). */
  readonly format: (value: number) => string;
  /**
   * Degree-tick count on the slider (data-driven per frame row; omitted =
   * no ticks). Ticks render at i/(n+1) of the track — the frame's evenly
   * stepped degree marks, with the default value sitting exactly on one.
   */
  readonly ticks?: number;
}

export interface SliderRowProps<TKey extends string> {
  /** Row definition (label, range, format, ticks). */
  readonly row: SliderRowDef<TKey>;
  /** Current value (card-local state). */
  readonly value: number;
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
  /** Resolves the row's label key (the lane's useMsg seam). */
  readonly resolveLabel: (key: TKey) => string;
}

/** Renders one label/value/slider row (see module header for the grammar). */
function SliderRowInner<TKey extends string>({
  row,
  value,
  onChange,
  resolveLabel,
}: SliderRowProps<TKey>): ReactElement {
  const label = resolveLabel(row.labelKey);
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
              // w26.i07: the marks live on the FILL domain (2px inset +
              // (track − 4px)·i/(n+1)), exactly where the frames draw them
              // ("Audio - Spatializer Settings.svg" Size row: x = 38 +
              // 324·i/11 — 67.45…332.55 on the x36–364 track; the Essential
              // gamma/banding + Colors sepia rows measure the same domain)
              // and where the flush thumb travels — the old full-width
              // percent drifted up to 1.7px (the video copy's pre-k10 bug).
              // `percent` shares the domain, so the covered-tick band below
              // compares like with like.
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
                  style={{ left: `calc(2px + (100% - 4px) * ${left.toFixed(3)} / 100)` }}
                />
              );
            })}
          </div>
        ) : null}
      </div>
    </div>
  );
}

/**
 * The shared row, memoized (j10 P3-5 / j11 P3-5): rows re-render only when
 * their own `value` changes — a per-move `setValues` in the host card no
 * longer re-runs every row's i18n hook + tick math.
 */
export const SliderRow = memo(SliderRowInner) as typeof SliderRowInner;
