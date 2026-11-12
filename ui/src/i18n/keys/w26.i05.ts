// w26.i05 lane keys (h07 fix cycle — the customize family's consumer half).
//
// The real ControlBar's toolbar items (customize/toolbarItems.tsx, wired by
// the Lead into ControlBar.tsx per the lane log) and the h07 state-7 seam
// copy. Registered into ui/src/i18n/messages.ts by the Lead after the wave
// merge (contract §4.5); until then toolbarItems resolves these via its
// seam-first local resolver, so the copy renders identically before and
// after. Sentence case per contract §2.
export const keys = {
  // The real bar's customized cluster (aria group over the placed items —
  // frame 7: the dropped tiles live in the real transport band).
  "customize.toolbar.label": "Custom toolbar items",
  // Honest disabled reason for UI-owned tiles with no real surface in this
  // shell (favorite has no engine command; quit's window close is
  // host-owned; the text-block family is palette-only) — never theater,
  // the PlayerScreen quit-row precedent.
  "customize.toolbar.unavailable": "Not available yet",
  // The band preview's seek-row timecodes are decorative chrome
  // (aria-hidden, frame-pinned values) — no keys needed for them.
} as const;
// Allowlist note (registry lives in ui/i18n-allowlist.json — appended by this
// lane): the frame-pinned band timecodes "00:02:10" / "00:25:40" are
// decorative aria-hidden preview chrome (Current Time.svg / Duration.svg's
// exact reads), not translatable copy — the real bar renders the live
// localized values.
