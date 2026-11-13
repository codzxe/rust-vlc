// w26.k16 lane keys (onboarding fixer pass, j16 findings #13/#14) — the two
// strings the review flagged for catalog flow (contract §4.5): the
// subtitle-languages placeholder (P3-13: raw "en, de" literal bypassing
// useMsg) and the step-progress template (P3-14: "Onboarding step 1 of 8"
// was composed by concatenation across two keys — a pattern that cannot
// survive word-order-different locales). Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge; until then the
// onboarding screen resolves these through its local seam-first resolver
// (see OnboardingScreen.tsx `useOnboardingMsg`), so the copy renders
// identically before and after registration.
export const keys = {
  // Placeholder for the preferred subtitle languages field — the settings
  // row's own example value (comma-separated language codes).
  "onboarding.subtitles.languages.placeholder": "en, de",
  // Step-progress announcement, one template so word order can vary per
  // locale ({current}/{total} are filled by formatStepLabel).
  "onboarding.progress.template": "Onboarding step {current} of {total}",
} as const;
