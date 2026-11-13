// w26.k19 lane keys (CONTRACT §4.5 — the Lead registers these into
// messages.ts post-wave; until then consumers render the lane copy through
// the seam-first resolver, identical before and after registration).
//
// One new string this lane: the document-title template (j19 J19-7 —
// route-level document.title, WCAG 2.4.2 in spirit for an SPA). The screen
// names themselves REUSE existing msgids (nav.*, shortcuts.title,
// stream.title, discovery.title, convert.title, settings.window-title.*,
// not-found.heading, app.name) — no per-screen copy invented here.
export const keys = {
  "app.document-title.template": "{screen} — Lumen",
} as const;
