// w26.f05 — audio/EQ anatomy lane. Genuinely new keys (CONTRACT §4.5): the
// copy for the A2 vertical-fader EQ panel (frame `EQ.svg` / the
// `Settings - audio-1.svg` embed). The Lead registers these into
// ui/src/i18n/messages.ts after the wave merge; until then the equalizer
// module resolves them via its lane-key seam (useEqMsg.ts, the w26.c1
// pattern), so the copy renders identically before and after registration.
//
// FX4-A08 (wave FX4, plan F1.1): the panel is CUT to the engine's 3-band
// truth (Eq3Band — the FIX-PLAN's "cut the UI to 3 bands and say so"
// ruling), so the morphing header's copy STATES the 3-band fact instead of
// the frame's unqualified "Equalizer" — the visible labels never carry a
// 10-band pretense.
//
// NOT duplicated here (reused — grep first, contract §4.5): every other EQ
// string lives in ui/src/i18n/keys/w26.c1.ts (band labels, presets, unit)
// and the tool-card toggle copy in ui/src/i18n/keys/w26.a18.ts.
export const keys = {
  // EQ.svg title-bar states — the collapsed 784×36 bar reads "Open
  // 3-band equalizer", the expanded panel header reads "Close 3-band
  // equalizer" (both centered with a chevron-∨; the 3-band qualifier is the
  // engine truth, FX4-A08).
  "screen.eq.open-panel": "Open 3-band equalizer",
  "screen.eq.close-panel": "Close 3-band equalizer",
} as const;
