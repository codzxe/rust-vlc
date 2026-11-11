// w26.c2 — dynamics tools lane (player audio dynamics: Compressor /
// Spatializer / Stereo Widener, frames "Audio - Compressor Settings.svg",
// "Audio - Spatializer Settings.svg", "Audio - Stereo Widener.svg").
// Genuinely new keys (CONTRACT §4.5). The Lead registers these into
// ui/src/i18n/messages.ts after the wave merge; until then the components
// resolve them seam-first (central catalog → this file) via the
// useDynamicsMsg resolver — the w26.a17 customize-panel pattern — so the
// copy renders identically before and after registration.
//
// The card-toggle labels are NOT duplicated here: "Expand Settings" /
// "Collapse Settings" are w26.a18's pre-registered
// player.tools.expand-settings / player.tools.collapse-settings (reuse per
// CONTRACT §4.5; the resolver falls back to that lane's key file).
export const keys = {
  // Tool names — the cards carry no visible title in the frames; these name
  // the accessible group + the vision-harness captions.
  "player.tools.dynamics.compressor": "Compressor",
  "player.tools.dynamics.spatializer": "Spatializer",
  "player.tools.dynamics.stereo-widener": "Stereo Widener",
  // Compressor rows (frame: Audio - Compressor Settings.svg). Row 1 reads
  // "FM5 / Peak" in the design — a typo for "RMS / Peak", corrected here per
  // the w26.a20 analysis (§A4 / §D4: i18n values use the correct words).
  "player.tools.compressor.rms-peak": "RMS / Peak",
  "player.tools.compressor.attack": "Attack",
  "player.tools.compressor.release": "Release",
  "player.tools.compressor.threshold": "Threshold",
  "player.tools.compressor.ratio": "Ratio",
  "player.tools.compressor.knee": "Knee Radius",
  "player.tools.compressor.makeup": "Makeup Gain",
  // Spatializer rows (frame: Audio - Spatializer Settings.svg).
  "player.tools.spatializer.size": "Size",
  "player.tools.spatializer.width": "Width",
  "player.tools.spatializer.wet": "Wet",
  "player.tools.spatializer.dry": "Dry",
  "player.tools.spatializer.damp": "Damp",
  // Stereo widener rows (frame: Audio - Stereo Widener.svg).
  "player.tools.widener.delay": "Delay Time",
  "player.tools.widener.feedback": "Feedback Gain",
  "player.tools.widener.crossfeed": "Crossfeed",
  "player.tools.widener.dry-mix": "Dry Mix",
  // Staged-tool note — the honest-unbound marker (effects-area pattern,
  // ui/src/screens/settings/areas/effects.tsx: visible, inert of engine
  // claims, names the command layer it awaits).
  "player.tools.dynamics.staged":
    "Staged — controls activate with the audio effects command card",
} as const;
