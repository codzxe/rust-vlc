// w26.c2 — ToolSliderRow: one dynamics-tool row, per the shared tool-card
// grammar (w26.a20 analysis §A4–A6/§D1, frames "Audio - Compressor
// Settings.svg" / "Audio - Spatializer Settings.svg" / "Audio - Stereo
// Widener.svg").
//
// w26.k10 (j10 P2-3): the row itself now lives in ONE place —
// tools/shared/SliderRow (generic over the label key, `resolveLabel` prop,
// the corrected w26.i07 fill-domain tick paint, React.memo). THIS module is
// the frozen public dynamics wrapper: same props (row: DynamicsRowDef,
// value, onChange), the label resolved through this lane's useDynamicsMsg
// seam. The video lane's VideoSliderRow composes the same shared row — the
// 1:1 copy that had already drifted (j11 P2-1) is gone as soon as it
// migrates (k11).
//
// Grammar (see shared/SliderRow for the frame measurements): label
// #6B6B6B ≈15px left · plain value right (the PLAIN-value family) ·
// full-width slider below: track 328×16 rx6 #090909, flat #371F06 fill
// 12px rx4 inset 2, thumb 8×24 rx4 #AB6314, optional degree ticks. The
// slider is the standard components/slider Slider (mission pin: not
// EQSlider). Fully controlled: local card state flows back in.

import type { ReactElement } from "react";
import { SliderRow } from "../shared";
import { useDynamicsMsg } from "./copy";
import type { DynamicsRowDef } from "./types";

export interface ToolSliderRowProps {
  /** Row definition (label, range, format, ticks). */
  readonly row: DynamicsRowDef;
  /** Current value (card-local state). */
  readonly value: number;
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
}

/** Renders one label/value/slider row (the shared row + this lane's i18n). */
export function ToolSliderRow({
  row,
  value,
  onChange,
}: ToolSliderRowProps): ReactElement {
  const m = useDynamicsMsg();
  return <SliderRow row={row} value={value} onChange={onChange} resolveLabel={m} />;
}
