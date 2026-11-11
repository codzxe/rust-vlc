// w26.b12 lane keys (discovery screen) — the split empty-state copy that has
// no existing catalog entry. Registered into ui/src/i18n/messages.ts by the
// Lead after the wave merge (contract §4.5); until then the screen resolves
// these via its local seam-first resolver (see DiscoveryScreen.tsx
// `useDiscoveryMsg`), so the copy renders identically before and after
// registration. All other discovery copy reuses the existing
// `discovery.*` catalog keys (grepped first).
export const keys = {
  // Quiet empty state (ListEmpty grammar: short title + explanatory body).
  // Split from the single legacy `discovery.volumes.empty` line so the quiet
  // state reads like the family's calm lists, not a paragraph.
  "discovery.volumes.empty-title": "No volumes reported yet",
  "discovery.volumes.empty-body":
    "Volume watching arrives with the platform integrations (issue #42).",
} as const;
