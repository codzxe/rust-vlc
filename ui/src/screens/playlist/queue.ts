// Card 4.D1 — queue semantics as PURE functions (doc 03 §4 playlist model +
// doc 05 §2.9 command table are the contract). UI-side order math lives here
// so both the provider (command dispatch) and the drag-reorder interaction
// share one tested implementation; the engine remains the source of truth.

import type { RepeatMode, SourceSpec } from "../../client/transport";

/** VLC-like repeat cycle: off → all → one → off (doc 08 §5.3 icons). */
export function cycleRepeat(mode: RepeatMode): RepeatMode {
  switch (mode) {
    case "off":
      return "all";
    case "all":
      return "one";
    case "one":
      return "off";
  }
}

/**
 * Play-next insertion index (doc 03 §810: "Play next" is an insertion into
 * the remaining order, not a mode): immediately after the current item;
 * with no current item it appends (nothing is "next").
 */
export function playNextTargetIndex(currentIndex: number, length: number): number {
  if (currentIndex < 0) return length;
  return Math.min(currentIndex + 1, length);
}

/**
 * Convert a drag-and-drop "insert before absolute index `before`" gesture on
 * the pre-move list into the `to_index` of `playlist.move` (post-removal
 * coordinates). Doc 05: `move` takes the id + target index; removing shifts.
 */
export function dragToMoveIndex(before: number, from: number, length: number): number {
  const clamped = Math.max(0, Math.min(before, length));
  const afterRemoval = clamped > from ? clamped - 1 : clamped;
  return Math.max(0, Math.min(afterRemoval, length - 1));
}

/** Index of `id` in `ids`, or -1 when absent (ids, not indices, are stable). */
export function indexOfId(ids: readonly number[], id: number): number {
  return ids.indexOf(id);
}

/** m:ss / h:mm:ss; `null` stays `null` (honest unknown, doc 08 §5). */
export function formatDuration(seconds: number | null): string | null {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) return null;
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m);
  return `${h > 0 ? `${h}:` : ""}${mm}:${String(s).padStart(2, "0")}`;
}

/** Human location string for an entry's source spec (M3U location column). */
export function sourceLocation(source: SourceSpec | null): string | null {
  if (!source) return null;
  return source.kind === "file" ? source.path : source.url;
}
