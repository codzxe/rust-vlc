// FX4-A08 (wave FX4, plan F1.1) — the equalizer's band table, CUT TO THE
// ENGINE'S TRUTH. The pre-wave table was VLC's classic 10-band set
// (frame-verbatim labels for a 10-band pretense); the engine's real
// equalizer is `Eq3Band` — three fixed bands (crates/audio-engine/src/
// effects/eq.rs: low 250 Hz shelf · mid 1 kHz peak · high 4 kHz shelf) —
// and the FIX-PLAN's ruling for the UI is explicit: cut to 3 bands and
// say so, never invent the 7 missing bands. Labels therefore carry the
// engine's own corner/center frequencies — the visible copy states the
// 3-band truth (open/close header, GraphicEQ's module header).
//
// Monochrome columns (w26.e07 F8, standing): the frames render the band
// columns monochrome — no per-band accent dots exist in any shipped
// anatomy, so the band table carries no accent data (w26.k10, j10 P2-2:
// the dead `accent` field is gone and stays gone).

import type { EqMessageKey } from "./useEqMsg";

/** One EQ band: stable id + i18n label. */
export interface EqBandSpec {
  /** Stable band id (also the React key). */
  readonly id: string;
  /** Column label key — engine band truth, frequencies included. */
  readonly labelKey: EqMessageKey;
}

/**
 * The engine's three bands, low → high (Eq3Band's fixed corners — the
 * UI labels them with the engine's own frequencies; there is no fourth).
 */
export const EQ_BANDS: readonly EqBandSpec[] = [
  { id: "low", labelKey: "screen.eq.band.low" },
  { id: "mid", labelKey: "screen.eq.band.mid" },
  { id: "high", labelKey: "screen.eq.band.high" },
];

/**
 * Where each engine band's gain lives in an `EqualizerState.gains` array —
 * exactly `EQ_BANDS` order (low, mid, high). The dispatch seam
 * (PlayerActions.setEq) reads gains through these indices so a state array
 * and the wire payload can never disagree about which band is which.
 */
export const EQ_BAND_INDEX = {
  low: 0,
  mid: 1,
  high: 2,
} as const;
