// w26.i16 lane keys (menus/dropdowns/profiles family, w26.h18) — the Profile
// flyout's copy (Profiles.png / analysis §A10). Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge (contract §4.5);
// until then the host resolves these via the lane seam-first resolver (the
// ControlBar mount proposal in docs/waves/w26/logs/w26.i16.md), so the copy
// renders identically before and after registration.
export const keys = {
  // Profiles.svg flyout row 5 — the plus glyph carries the "+", the label
  // reads "New" (analysis §A10: `row 5 "New": plus icon 14×14 #8F8F8F`).
  "player.appmenu.profile.new": "New",
  // Quick-create base name for the "+ New" row (the store appends the
  // number): "Profile 5". Local device data — same posture as the seeded
  // saved-playlist names, no engine claim.
  "player.appmenu.profile.auto-name": "Profile",
} as const;
