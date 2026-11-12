// w26.i04 lane keys (player variants family — the miniplayer queue's
// enqueue seam) — copy that has no existing catalog entry. Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge (contract §4.5);
// until then the mini queue resolves these via the lane seam-first resolver
// (see MiniPlayer.tsx `useLaneMsg`), so the copy renders identically before
// and after registration. Source: Miniplayer.png queue artboards (the "+"
// add affordance on the queue meta row).
export const keys = {
  // The mini queue panel's "+" trigger — opens the add-to-queue popup (the
  // frame's plus affordance, now wired to the real playlist.enqueue seam).
  "player.mini.queue.add": "Add to queue",
  // Honest empty row while no pickable source exists (desktop shell with an
  // empty opening history — an inert row, never a fabricated source).
  "player.mini.queue.add-empty": "Nothing to add yet",
} as const;
