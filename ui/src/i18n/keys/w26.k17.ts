// w26.k17 lane keys (CONTRACT §4.5 — the Lead registers these into
// messages.ts post-wave; until then consumers render the lane copy through
// the seam-first resolver, identical before and after registration).
//
// One new string this lane: the honest host-unreachable note for
// /media-info (j17 P3-8) — a dead transport is a DIFFERENT verdict from
// "the command layer did not expose it"; the panel must never conflate
// them.
export const keys = {
  "media-info.panel.unreachable":
    "Media information is unavailable — the engine is not reachable.",
} as const;
