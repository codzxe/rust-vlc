// w26.c4 — VideoSliderRow: one video-tool slider row. w26.k11 (j11 P2-1 /
// P3-5): the row itself now lives in ONE place — tools/shared/SliderRow
// (w26.k10), generic over the label key with a `resolveLabel` prop — which
// this lane composes instead of the former 1:1 copy of the dynamics
// ToolSliderRow. The copy had already drifted: it kept the pre-i07
// full-width tick percent while every multi-tick video frame draws its
// degree marks on the 2px-inset FILL domain (Essential gamma tick 1 painted
// −1.6px off "Video - Essential.svg"; grain ±1.6, banding ±1.4, sharpen ±1,
// the 2-tick rows ±0.7). The shared row paints the corrected i07 domain and
// is React.memo'd — a drag on one row no longer re-renders the card's other
// rows (j11 P3-5).
//
// Grammar (w26.a20 analysis §B1/§B3/§D1, frames "Video - Colors.svg" /
// "Video - Essential.svg"): label #6B6B6B ≈15px left, NO value column (both
// frames' slider rows are label-only — video.css hides the shared value
// span; the slider's aria-valuetext still reports every value), full-width
// slider below: track 328×16 rx6 #090909, flat #371F06 fill 12px rx4 inset 2,
// thumb 8×24 rx4 #AB6314, optional degree ticks (6px marks on the track
// centerline, drawn over the fill with the thumb over its own tick — the
// frame z-order). See shared/SliderRow for the tick-domain measurements.

import type { ReactElement } from "react";
import { SliderRow } from "../shared";
import { useVideoMsg } from "./copy";
import type { VideoSliderRowDef } from "./types";

export interface VideoSliderRowProps {
  /** Row definition (label, range, format, ticks). */
  readonly row: VideoSliderRowDef;
  /** Current value (card-local state). */
  readonly value: number;
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
}

/** Renders one label/slider row (the shared row + this lane's i18n seam). */
export function VideoSliderRow({
  row,
  value,
  onChange,
}: VideoSliderRowProps): ReactElement {
  const m = useVideoMsg();
  return <SliderRow row={row} value={value} onChange={onChange} resolveLabel={m} />;
}
