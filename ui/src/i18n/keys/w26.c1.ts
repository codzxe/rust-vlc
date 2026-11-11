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
// text. Band labels are frame-verbatim, including the frames' "KHz"
// capitalization ("1 KHz" — the VLC-style unit, not SI "kHz"); the frames
// win over the general sentence-case rule (contract §3). Preset names use
// correct English per the w26.a20 analysis §D4 ("Full bass", not the render
// mis-read "Full base"; "Full bass & treble" per EQ presets.svg row 7).
export const keys = {
  // The card's accessible name (settings.audio-1 row context: "Equalizer").
  "screen.eq.title": "Equalizer",
  // Row labels — frame: Audio - Equalizer.svg rows 1–11 (label column).
  "screen.eq.preamp": "Preamp",
  "screen.eq.band.60": "60 Hz",
  "screen.eq.band.170": "170 Hz",
  "screen.eq.band.310": "310 Hz",
  "screen.eq.band.600": "600 Hz",
  "screen.eq.band.1k": "1 KHz",
  "screen.eq.band.3k": "3 KHz",
  "screen.eq.band.6k": "6 KHz",
  "screen.eq.band.12k": "12 KHz",
  "screen.eq.band.14k": "14 KHz",
  "screen.eq.band.16k": "16 KHz",
  // Value unit (chips read "12.0 dB" — frame: Audio - Equalizer.svg).
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
