/** Public surface of the equalizer tool (w26.c1; w26.f05 A2 anatomy;
 * FX4-A08 3-band truth + real audio.set-eq dispatch) — the GraphicEQ
 * vertical-fader panel (EQ.svg), the EqualizerTool that composes it with
 * the preset selector for the tools dialog (dispatching the real verb
 * through the player context), the band + preset tables, and the default
 * state payload for the engine wire-up. */
export { EqualizerTool } from "./EqualizerTool";
export type { EqualizerToolProps, EqualizerState } from "./EqualizerTool";
export { GraphicEQ } from "./GraphicEQ";
export type { GraphicEQProps } from "./GraphicEQ";
export { useEqMsg } from "./useEqMsg";
export type { EqMessageKey } from "./useEqMsg";
export { EQ_BANDS, EQ_BAND_INDEX } from "./bands";
export type { EqBandSpec } from "./bands";
export {
  DEFAULT_EQUALIZER_STATE,
  DEFAULT_PREAMP_DB,
  EQ_GAIN_MAX_DB,
  EQ_GAIN_MIN_DB,
  EQ_GAIN_STEP_DB,
  EQ_PRESETS,
  NEUTRAL_GAINS,
} from "./presets";
export type { EqPresetSpec } from "./presets";
