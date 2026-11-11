// w26.c1 — the EQ preset table. Frame: `EQ presets.svg` state=open (8 rows,
// row 8 clipped by the 348px list — the list scrolls, w26.a20 §A3) +
// `Settings - audio-1.svg` ("Equalizer preset" row).
//
// Gain values are the classic Winamp/VLC 10-band preset shapes (dB, band
// order = EQ_BANDS low → high). The frames outline the labels but carry no
// numeric preset data — the values below are the canonical shapes VLC ships
// [VERIFY against the engine lane once an equalizer command card exists].
// VLC additionally ships Headphones / Large Hall / Live / Party / Pop /
// Reggae / Rock / Ska / Soft / Soft rock / Techno presets — out of the
// frame's 8-row scope, logged for the Lead (the scrollable Dropdown list
// takes them without layout changes if wanted).

import type { EqMessageKey } from "./useEqMsg";
import type { EqualizerState } from "./EqualizerTool";

/** One EQ preset: stable id, i18n label, 10 band gains in dB. */
export interface EqPresetSpec {
  /** Stable preset id (the Dropdown option value). */
  readonly id: string;
  /** Option label key (frame: EQ presets.svg rows 1–8). */
  readonly labelKey: EqMessageKey;
  /** 10 band gains in dB, band order = EQ_BANDS. */
  readonly gains: readonly number[];
}

/** Neutral gains — "None" and "Flat" both sit at 0 dB (frame rows 1–2). */
export const NEUTRAL_GAINS: readonly number[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/** The frame's 8 presets, frame row order. */
export const EQ_PRESETS: readonly EqPresetSpec[] = [
  { id: "none", labelKey: "screen.eq.preset.none", gains: NEUTRAL_GAINS },
  { id: "flat", labelKey: "screen.eq.preset.flat", gains: NEUTRAL_GAINS },
  {
    id: "classical",
    labelKey: "screen.eq.preset.classical",
    gains: [0, 0, 0, 0, 0, 0, -7.2, -7.2, -7.2, -9.6],
  },
  {
    id: "club",
    labelKey: "screen.eq.preset.club",
    gains: [0, 0, 4.8, 3.6, 3.6, 3.6, 2.4, 0, 0, 0],
  },
  {
    id: "dance",
    labelKey: "screen.eq.preset.dance",
    gains: [5.6, 3.5, 1.6, 0, 0, -3.2, -4.8, -5.6, -5.6, -5.6],
  },
  {
    id: "full-bass",
    labelKey: "screen.eq.preset.full-bass",
    gains: [-8, 9.6, 9.6, 5.6, 1.6, -4, -8, -10.4, -11.2, -11.2],
  },
  {
    id: "full-bass-treble",
    labelKey: "screen.eq.preset.full-bass-treble",
    gains: [4.8, 3.6, 0, -7.2, -4.8, 1.2, 8, 11.2, 12, 12],
  },
  {
    id: "full-treble",
    labelKey: "screen.eq.preset.full-treble",
    gains: [-9.6, -9.6, -9.6, -4, 2.4, 11.2, 16, 16, 16, 16.4],
  },
];

/** Default preamp in dB (frame: Audio - Equalizer.svg reads "12.0 dB"). */
export const DEFAULT_PREAMP_DB = 12;

/** The frame's default state: preamp 12.0 dB, flat bands, no preset. */
export const DEFAULT_EQUALIZER_STATE: EqualizerState = {
  preamp: DEFAULT_PREAMP_DB,
  gains: NEUTRAL_GAINS,
  preset: "none",
};

/**
 * Band/preamp slider range in dB. VLC's classic band-gain span is ±20 dB;
 * the frame's 0 dB band fills sit at ≈50.6% (symmetric range) and the
 * preamp's 12.0 dB fill at ≈82.6% — closest simple range is ±20 dB for both
 * (12 → 80%, Δ2.6% ≈ 8px) [VERIFY with the engine lane].
 */
export const EQ_GAIN_MIN_DB = -20;
export const EQ_GAIN_MAX_DB = 20;

/** Slider step in dB (frame values render one decimal, e.g. "12.0 dB"). */
export const EQ_GAIN_STEP_DB = 1;
