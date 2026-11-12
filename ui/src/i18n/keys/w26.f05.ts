// w26.f05 — audio/EQ anatomy lane. Genuinely new keys (CONTRACT §4.5): the
// copy for the A2 vertical-fader EQ panel (frame `EQ.svg` / the
// `Settings - audio-1.svg` embed). The Lead registers these into
// ui/src/i18n/messages.ts after the wave merge; until then the equalizer
// module resolves them via its lane-key seam (useEqMsg.ts, the w26.c1
// pattern), so the copy renders identically before and after registration.
//
// NOT duplicated here (reused — grep first, contract §4.5): every other EQ
// string lives in ui/src/i18n/keys/w26.c1.ts (band labels, presets, unit)
// and the tool-card toggle copy in ui/src/i18n/keys/w26.a18.ts.
export const keys = {
  // EQ.svg title-bar states — the collapsed 784×36 bar reads "Open
  // Equalizer", the expanded 784×489 panel header reads "Close Equalizer"
  // (both centered with a chevron-∨; frame-verbatim, sentence case).
  "screen.eq.open-panel": "Open Equalizer",
  "screen.eq.close-panel": "Close Equalizer",
} as const;
