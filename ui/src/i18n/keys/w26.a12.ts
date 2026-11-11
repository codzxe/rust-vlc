// w26.a12 lane keys (onboarding) — the W26 Figma wizard copy that has no
// catalog entry yet. Registered into ui/src/i18n/messages.ts by the Lead
// after the wave merge (contract §4.5); until then the onboarding screen
// resolves these via its local seam-first resolver (see OnboardingScreen.tsx
// `useOnboardingMsg`), so the copy renders identically before and after
// registration. Frame-provenance notes per key.
export const keys = {
  // Media/library step (frame: Onboarding.svg "Choose a Media Folder…"
  // input row, recomposed around this build's real open-path funnel).
  "onboarding.library.title": "Media library",
  "onboarding.library.body":
    "Open a file, a playlist, or a network stream — paste a path or URL to queue it now, or leave it empty and open media from the player later.",
  "onboarding.library.opened": "Opened — it will be waiting when you finish.",
  "onboarding.library.failed": "That path could not be opened. Check it and try again.",
  // Appearance step heading (frame: Onboarding-1.svg, verbatim).
  "onboarding.appearance.heading": "Select your preferred theme and color.",
  // Playback step (composed; controls are the real settings schema rows).
  "onboarding.playback.title": "Playback",
  "onboarding.playback.body":
    "Quiet, honest defaults — every change saves the moment you make it and stays editable in Settings.",
  // Subtitles step (composed; controls are the real settings schema rows).
  "onboarding.subtitles.title": "Subtitles",
  "onboarding.subtitles.body":
    "How subtitle tracks behave when a file carries them — saved the moment you change it, editable in Settings.",
  // Shortcuts step (frame: Onboarding-2.svg — heading + footer note verbatim).
  "onboarding.shortcuts.heading": "Learn the basic keyboard shortcuts.",
  "onboarding.shortcuts.more": "Find the full list in the menu.",
  // Pair-row labels for the two-column tour (frame: Onboarding-2.svg rows;
  // each row's keycaps come from the real keymap, never typed copy).
  "onboarding.shortcuts.seek": "Seek backward / forward",
  "onboarding.shortcuts.item": "Previous / next item",
  "onboarding.shortcuts.frame": "Previous / next frame",
  "onboarding.shortcuts.volume": "Increase / decrease volume",
} as const;
