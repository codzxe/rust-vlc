// w26.f02 lane keys (SettingField row anatomy) — the ⓘ hint affordance copy
// that has no catalog entry yet. Registered into ui/src/i18n/messages.ts by
// the Lead after the wave merge (contract §4.5); until then SettingField
// resolves it through its local seam-first resolver (see SettingField.tsx
// `useFieldMsg`), so the copy renders identically before and after
// registration. Frame provenance per key.
export const keys = {
  // ⓘ accessible name (§D2 row grammar: "optional ⓘ r7.5 #404040 sw1.125
  // after the label" — the frames pin the circle glyph, not the string).
  "settings.field.more-info": "More info",
} as const;
