// w26.b13 lane keys (convert / media-info / not-found / open-media) — the
// W26-redesign copy with no existing catalog entry. Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge (contract §4.5);
// until then the screens resolve them via their local seam-first resolvers,
// so the copy renders identically before and after registration. All other
// screen copy reuses the existing convert.* / media-info.* / not-found.* /
// open.* catalog keys (grepped first).
export const keys = {
  // /convert — the Settings-frame "input + Browse" row. No catalog command
  // exposes a native file picker yet (the desktop shell's dialog seam is a
  // Rust-side trait, apps/desktop files.rs, with no UI event), so the action
  // renders as the honest disabled-from-data stub (extensions.tsx /
  // StageOverlays data-wiring-stub precedent) named by this caption.
  "convert.source.browse": "Browse",
  "convert.source.browse-gate":
    "Native file browsing is not exposed in this build — paste a path instead.",
  // Quiet count suffix in the queue section header's trailing slot
  // (sidebar.svg "2 items" meta-row grammar, discovery count precedent).
  "convert.queue.count-suffix": "jobs",
  // 404 — the quiet W26 hero copy (CONTRACT §3: no direct frame; the
  // Solstice "Lost the thread" title dies with the v2 aesthetic). Distinct
  // key names so Lead registration cannot collide with the legacy
  // not-found.title/body strings; not-found.action ("Back to the player")
  // is reused as-is.
  "not-found.heading": "Page not found",
  "not-found.body-quiet": "The page you were looking for does not exist.",
} as const;
