// w26.c1 — the 10-band equalizer's band table (VLC's classic 10-band set,
// frame-verbatim labels). Frame: `Audio - Equalizer.svg` rows 2–11 (row 1 is
// Preamp, which is not a band) and `EQ.svg`'s 11 fader columns.
//
// Monochrome columns (w26.e07 F8, standing): the frames render the band
// columns monochrome (#6B6B6B labels, #AB6314 thumbs) — NO per-band accent
// dots exist in any shipped anatomy. The `--color-eq-1..7` tokens stay in
// the token layer for the visualizer's live spectrum bars (VisualizerDisplay
// derives its own `(index % 7) + 1`), so the band table carries no accent
// data. w26.k10 (j10 P2-2): the dead `accent` field — never read by any
// consumer — and this header's retired accent-dot paragraph are gone.

import type { EqMessageKey } from "./useEqMsg";

/** One EQ band: stable id + i18n label. */
export interface EqBandSpec {
  /** Stable band id (also the React key). */
  readonly id: string;
  /** Row label key (frame-verbatim "60 Hz" … "16 KHz"). */
  readonly labelKey: EqMessageKey;
}

/** VLC's classic 10-band set, low → high (frame row order). */
export const EQ_BANDS: readonly EqBandSpec[] = [
  { id: "hz60", labelKey: "screen.eq.band.60" },
  { id: "hz170", labelKey: "screen.eq.band.170" },
  { id: "hz310", labelKey: "screen.eq.band.310" },
  { id: "hz600", labelKey: "screen.eq.band.600" },
  { id: "khz1", labelKey: "screen.eq.band.1k" },
  { id: "khz3", labelKey: "screen.eq.band.3k" },
  { id: "khz6", labelKey: "screen.eq.band.6k" },
  { id: "khz12", labelKey: "screen.eq.band.12k" },
  { id: "khz14", labelKey: "screen.eq.band.14k" },
  { id: "khz16", labelKey: "screen.eq.band.16k" },
];
