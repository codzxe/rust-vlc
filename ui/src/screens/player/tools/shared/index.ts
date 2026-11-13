/** Public surface of the shared audio/video tool row module (w26.k10) —
 * the ONE slider row the dynamics (ToolSliderRow) and video (VideoSliderRow)
 * tools compose (j10 P2-3 / j11 P2-1: the drifted 1:1 copies consolidated).
 * The `.dynamics-*` row grammar it renders is styled by
 * tools/dynamics/dynamics.css — the shared sheet both lanes import. */
export { SliderRow } from "./SliderRow";
export type { SliderRowDef, SliderRowProps } from "./SliderRow";
