// w26.c3 — visualizer tool lane (player audio visualizer, frame
// "Audio Visualizer.svg" = the mode dropdown: Disabled / Spectrometer / Scope /
// Spectrum / Waveform). Genuinely new keys (CONTRACT §4.5). The Lead registers
// these into ui/src/i18n/messages.ts after the wave merge; until then the
// components resolve them seam-first (central catalog → this file) via the
// useVisualizerMsg resolver — the w26.c1 useEqMsg pattern — so the copy
// renders identically before and after registration.
//
// The card-toggle labels are NOT duplicated here: "Expand Settings" /
// "Collapse Settings" are w26.a18's pre-registered
// player.tools.expand-settings / player.tools.collapse-settings (reuse per
// CONTRACT §4.5; the resolver falls back to that lane's key file).
//
// Frame spelling note: the design renders "Spectometer" (sic) — the correct
// VLC word "Spectrometer" ships instead, the same correction policy as
// w26.c1's "RMS / Peak" and "Full bass" (w26.a20 §A7 / §D4).
export const keys = {
  // Tool name — the card's accessible group name + harness caption (the
  // frames carry no visible card title).
  "player.tools.visualizer.title": "Audio Visualizer",
  // The mode selector's accessible name (the frame's dropdown trigger).
  "player.tools.visualizer.mode": "Visualizer",
  // The five frame modes, in frame order.
  "player.tools.visualizer.mode.disabled": "Disabled",
  "player.tools.visualizer.mode.spectrometer": "Spectrometer",
  "player.tools.visualizer.mode.scope": "Scope",
  "player.tools.visualizer.mode.spectrum": "Spectrum",
  "player.tools.visualizer.mode.waveform": "Waveform",
  // Display state descriptions — the honest data markers woven into the
  // display's aria-label (no visible copy: the frames carry none).
  "player.tools.visualizer.display.resting": "Waiting for audio data",
  "player.tools.visualizer.display.live": "Live",
  // Staged-tool note — the honest-unbound marker (effects-area pattern,
  // settings/areas/effects.tsx + the dynamics cards): names the command
  // layer the mode selection awaits.
  "player.tools.visualizer.staged":
    "Staged — the visualizer activates with the audio effects command card",
} as const;
