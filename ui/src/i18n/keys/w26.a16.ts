// w26.a16 lane keys (player variants: miniplayer + music mode) — copy that
// has no existing catalog entry. Registered into ui/src/i18n/messages.ts by
// the Lead after the wave merge (contract §4.5); until then the variant
// screens resolve these via their local seam-first resolver (see
// MiniPlayer.tsx `useLaneMsg`), so the copy renders identically before and
// after registration. Sources: Miniplayer.png / Player - Music.png frames.
export const keys = {
  // Title row of the mini card while no session is up (honest — never a
  // fabricated media name).
  "player.mini.title-empty": "Nothing playing",
  // The collapsible queue panel above the video area (Miniplayer.png
  // artboards 2/4).
  "player.mini.queue.label": "Queue",
  // Meta-row toggle that opens/collapses the queue panel.
  "player.mini.queue.toggle": "Toggle queue",
} as const;
