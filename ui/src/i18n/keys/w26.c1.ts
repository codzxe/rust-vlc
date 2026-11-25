// w26.c1 — equalizer suite lane. Genuinely new keys introduced this lane
// (CONTRACT §4.5): the EQ tool card's copy. The Lead registers these into
// ui/src/i18n/messages.ts after the wave merge; until then the equalizer
// tool resolves them via its lane-key seam (useEqMsg.ts, the w26.b10
// useOptionsMsg pattern), so the copy renders identically before and after
// registration.
//
// Reused (NOT re-declared here — grep first, contract §4.5): the card
// toggle copy "player.tools.expand-settings"/"player.tools.collapse-settings"
// is owned by ui/src/i18n/keys/w26.a18.ts and resolved through the same seam.
//
// Copy provenance: `Audio - Equalizer.svg` / `EQ.svg` carry REAL outlined
// text. The band labels were the frame's 10-band set; FX4-A08 (wave FX4,
// plan F1.1) CUT the panel to the engine's 3-band truth (Eq3Band: low
// 250 Hz shelf · mid 1 kHz peak · high 4 kHz shelf) per the FIX-PLAN's
// "cut the UI to 3 bands and say so" ruling — the labels now state the
// engine's own frequencies, no invented bands. Preset names keep the
// correct English per the w26.a20 analysis §D4 ("Full bass", not the
// render mis-read "Full base"; "Full bass & treble" per EQ presets.svg
// row 7).
export const keys = {
  // The card's accessible name (settings.audio-1 row context: "Equalizer").
  "screen.eq.title": "Equalizer",
  // Row labels — the ENGINE's three bands, frequencies stated (FX4-A08).
  "screen.eq.preamp": "Preamp",
  "screen.eq.band.low": "Low (250 Hz)",
  "screen.eq.band.mid": "Mid (1 kHz)",
  "screen.eq.band.high": "High (4 kHz)",
  // Value unit (chips read "0.0 dB" flat — the engine's unity default).
  "screen.eq.unit-db": "dB",
  // Preset selector (frame: EQ presets.svg trigger; settings audio-1 row
  // label "Equalizer preset" for the accessible name).
  "screen.eq.preset": "Equalizer preset",
  // EQ presets.svg rows 1–8 (vision-confirmed labels; row 8 clips in the
  // frame — the list scrolls, w26.a20 §A3).
  "screen.eq.preset.none": "None",
  "screen.eq.preset.flat": "Flat",
  "screen.eq.preset.classical": "Classical",
  "screen.eq.preset.club": "Club",
  "screen.eq.preset.dance": "Dance",
  "screen.eq.preset.full-bass": "Full bass",
  "screen.eq.preset.full-bass-treble": "Full bass & treble",
  "screen.eq.preset.full-treble": "Full treble",
} as const;
