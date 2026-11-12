// w26.i18 (wave I fixer, lane 18/20 — working screens: /discovery, /convert,
// /media-info, 404; h19 ratings close-out). New message keys per CONTRACT
// §4.5 — the Lead registers them into ui/src/i18n/messages.ts post-wave;
// until then the screens' lane seam-first resolvers (useConvertMsg) resolve
// them from this file, so the copy renders identically before and after.
//
// Content rule: only genuinely NEW copy lives here. The media-info empty
// CTA reuses the registered "player.empty.open" ("Open media") and the
// discovery empty CTA reuses the c6 "discovery.menu.add-file" ("Add file…")
// — both grepped against the catalog first.
export const keys = {
  // /convert — the h19 §4 fix: "a designer with a spine would disable the
  // queue lane itself until the engine seam exists and say so once". This is
  // the once: the queue card's gate line, wired to the disabled primary
  // action via aria-describedby. Distinct key name so Lead registration
  // cannot collide with the legacy convert.lane.unavailable string (the
  // ritual copy: "Queued jobs report this honestly instead of pretending to
  // run" — no jobs can be queued anymore, so that sentence is gone).
  "convert.lane.gate":
    "This build exposes no conversion commands: the engine’s conversion lane is not wired into its command catalog yet, so queueing is disabled.",
} as const;
