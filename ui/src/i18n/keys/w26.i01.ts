// w26.i01 lane keys (onboarding fidelity closes) — frame copy that has no
// catalog entry yet. Registered into ui/src/i18n/messages.ts by the Lead
// after the wave merge (contract §4.5); until then the onboarding screen
// resolves these via its local seam-first resolver (OnboardingScreen.tsx
// `useOnboardingMsg`, which merges this catalog with w26.a12's), so the copy
// renders identically before and after registration. Frame provenance per key.
export const keys = {
  // Library step (frame: Onboarding.svg "Choose a Media Folder to get
  // started with" + Browse 114×35). The frame's folder picker is recomposed
  // around this build's real open-path funnel (file / playlist / network
  // stream), so the visible label says "media", not "a Media Folder" — the
  // funnel it labels is wider than a folder picker (h01 #1: the frame's
  // visible field label must render, not hideLabel).
  "onboarding.library.field-label": "Choose media to get started with",
  // frame: Onboarding.svg Frame 141 — the 114×35 #1B1B1B submit, verbatim.
  "onboarding.library.browse": "Browse",
  // Shortcuts tour row labels — frame: Onboarding-2.svg, VERBATIM (h02 #2:
  // the hotkey catalog's three casing conventions collapse into the frame's
  // one; keycaps stay real keymap data, only the row labels change).
  "onboarding.shortcuts.row.play-pause": "Play / Pause",
  "onboarding.shortcuts.row.seek": "Seek Backward / Forward",
  "onboarding.shortcuts.row.item": "Previous / Next Item",
  "onboarding.shortcuts.row.frame": "Previous / Next Frame",
  "onboarding.shortcuts.row.subtitle-cycle": "Cycle Subtitle Track",
  "onboarding.shortcuts.row.audio-cycle": "Cycle Audio Track",
  "onboarding.shortcuts.row.fullscreen": "Toggle Fullscreen",
  "onboarding.shortcuts.row.volume": "Increase / Decrease Volume",
  "onboarding.shortcuts.row.queue": "Show / Hide Queue",
  "onboarding.shortcuts.row.file-browser": "Show / Hide File Browser",
  "onboarding.shortcuts.row.snapshot": "Take a Snapshot",
  "onboarding.shortcuts.row.record": "Record",
} as const;
