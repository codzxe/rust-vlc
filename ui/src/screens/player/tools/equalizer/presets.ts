// FX4-A08 (wave FX4, plan F1.1) — the EQ preset table, CUT TO THE ENGINE'S
// 3-BAND TRUTH. The pre-wave table carried the classic Winamp/VLC 10-band
// preset shapes for a UI that pretended ten bands; the engine's equalizer
// is `Eq3Band` (crates/audio-engine/src/effects/eq.rs: low 250 Hz shelf ·
// mid 1 kHz peak · high 4 kHz shelf, preamp a `Volume`) and the FIX-PLAN
// rules the reconciliation: "cut the UI to 3 bands and say so" — never
// invent the 7 missing bands.
//
// Preset gains below are the MECHANICAL projection of the classic 10-band
// shapes onto the engine's three bands — each engine band takes the mean
// of the classic gains inside its own region (low: 60/170 Hz · mid:
// 310 Hz–3 kHz · high: 6–16 kHz), snapped to the 0.1 dB lattice. The
// frame (EQ presets.svg) outlines the 8 labels only; the projection is
// this module's own derivation and every value lands in the engine
// exactly as sent (AudioControls::set_eq — the host verb routes it).
// VLC's out-of-frame presets (Headphones / Large Hall / …) stay out.

import type { EqMessageKey } from "./useEqMsg";
import type { EqualizerState } from "./EqualizerTool";

/** One EQ preset: stable id, i18n label, 3 band gains in dB. */
export interface EqPresetSpec {
  /** Stable preset id (the Dropdown option value). */
  readonly id: string;
  /** Option label key (frame: EQ presets.svg rows 1–8). */
  readonly labelKey: EqMessageKey;
  /** 3 band gains in dB, band order = EQ_BANDS (low, mid, high). */
  readonly gains: readonly number[];
}

/** Neutral gains — "None" and "Flat" both sit at 0 dB (the engine identity). */
export const NEUTRAL_GAINS: readonly number[] = [0, 0, 0];

/** The frame's 8 presets, frame row order, projected to the 3 bands. */
export const EQ_PRESETS: readonly EqPresetSpec[] = [
  { id: "none", labelKey: "screen.eq.preset.none", gains: NEUTRAL_GAINS },
  { id: "flat", labelKey: "screen.eq.preset.flat", gains: NEUTRAL_GAINS },
  {
    id: "classical",
    labelKey: "screen.eq.preset.classical",
    gains: [0, -1.8, -7.8],
  },
  {
    id: "club",
    labelKey: "screen.eq.preset.club",
    gains: [0, 3.9, 0.6],
  },
  {
    id: "dance",
    labelKey: "screen.eq.preset.dance",
    gains: [4.6, -0.4, -5.4],
  },
  {
    id: "full-bass",
    labelKey: "screen.eq.preset.full-bass",
    gains: [0.8, 3.2, -10.2],
  },
  {
    id: "full-bass-treble",
    labelKey: "screen.eq.preset.full-bass-treble",
    gains: [4.2, -2.7, 10.8],
  },
  {
    id: "full-treble",
    labelKey: "screen.eq.preset.full-treble",
    gains: [-9.6, 0, 16.1],
  },
];

/**
 * Default preamp in dB. The engine's uncontrolled chain is the exact
 * identity (`Volume::UNITY` — a flat EQ is bit-exact passthrough, eq.rs's
 * pinned property), so the honest default is 0 dB, not the retired frame
 * chip's "12.0" (a +12 dB trim on every quantum by default — loudness the
 * engine was never asked for).
 */
export const DEFAULT_PREAMP_DB = 0;

/** The default state: preamp 0 dB (unity), flat bands, no preset. */
export const DEFAULT_EQUALIZER_STATE: EqualizerState = {
  preamp: DEFAULT_PREAMP_DB,
  gains: NEUTRAL_GAINS,
  preset: "none",
};

/**
 * Band/preamp slider range in dB — the UI design range. The ENGINE's
 * runtime control bound is ±40 dB (eq.rs `EQ_GAIN_DB_MAX`, armed by
 * FX3-A03); the UI stays inside it at the classic ±20 dB, and the engine
 * answers out-of-range values with the typed refusal (never clamped).
 */
export const EQ_GAIN_MIN_DB = -20;
export const EQ_GAIN_MAX_DB = 20;

/**
 * Slider step in dB — the keyboard/pointer LATTICE. The projected preset
 * gains are one-decimal (w26.k10, j10 P2-1) and the chips render one
 * decimal, so the lattice IS 0.1 dB: every arrow is ±0.1 (Shift/PageUp
 * coarse ±1.0), symmetric from any preset value, matching exactly where a
 * drag can land.
 */
export const EQ_GAIN_STEP_DB = 0.1;
