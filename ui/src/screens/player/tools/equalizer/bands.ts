// w26.c1 — the 10-band equalizer's band table (VLC's classic 10-band set,
// frame-verbatim labels). Frame: `Audio - Equalizer.svg` rows 2–11 (row 1 is
// Preamp, which is not a band) and `EQ.svg`'s 11 fader columns.
//
// Band accents: the Lead's wave pin ("band accents from the eq token group" —
// themes/tokens.json semantic color.eq.1..7, `--color-eq-1..7`, the W25 §1
// "EQ band set"). The frames themselves render monochrome rows (#6B6B6B
// labels, #AB6314 thumbs) — the per-band accent is a product-level identity
// marker (a 6px dot before the label) layered on the frame grammar; the 7
// tokens cycle across the 10 bands.

import type { EqMessageKey } from "./useEqMsg";

/** One EQ band: stable id, i18n label, accent token index (eq.1–eq.7). */
export interface EqBandSpec {
  /** Stable band id (also the React key). */
  readonly id: string;
  /** Row label key (frame-verbatim "60 Hz" … "16 KHz"). */
  readonly labelKey: EqMessageKey;
  /** `--color-eq-N` token index, 1–7. */
  readonly accent: 1 | 2 | 3 | 4 | 5 | 6 | 7;
}

/** VLC's classic 10-band set, low → high (frame row order). */
export const EQ_BANDS: readonly EqBandSpec[] = [
  { id: "hz60", labelKey: "screen.eq.band.60", accent: 1 },
  { id: "hz170", labelKey: "screen.eq.band.170", accent: 2 },
  { id: "hz310", labelKey: "screen.eq.band.310", accent: 3 },
  { id: "hz600", labelKey: "screen.eq.band.600", accent: 4 },
  { id: "khz1", labelKey: "screen.eq.band.1k", accent: 5 },
  { id: "khz3", labelKey: "screen.eq.band.3k", accent: 6 },
  { id: "khz6", labelKey: "screen.eq.band.6k", accent: 7 },
  { id: "khz12", labelKey: "screen.eq.band.12k", accent: 1 },
  { id: "khz14", labelKey: "screen.eq.band.14k", accent: 2 },
  { id: "khz16", labelKey: "screen.eq.band.16k", accent: 3 },
];
