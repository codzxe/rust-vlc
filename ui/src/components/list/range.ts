/**
 * Card 4.B4 — virtualization windowing math (pure, unit-testable).
 *
 * The standard fixed-height windowing approach, hand-rolled (zero-new-deps
 * rule, card 4.B4): given scrollTop + viewport height, render only the
 * visible rows plus `overscan` rows on each side. DOM node count stays
 * O(viewport / itemHeight) regardless of data size — a 10k-row list renders
 * ~25 row nodes, not 10k (asserted by the perf test). Absolute row
 * positioning inside a full-height spacer keeps the scrollbar native and
 * scroll-jump-exact.
 */

/** Windowed range computed by the virtualization math. */
export interface VirtualRange {
  startIndex: number;
  /** Inclusive last rendered index. */
  endIndex: number;
  /** Spacer height in px: count × itemHeight (exact scrollbar geometry). */
  totalHeight: number;
}

/**
 * The standard fixed-height windowing math, hand-rolled (zero-new-deps rule,
 * card 4.B4): given scrollTop + viewport height, render only the visible rows
 * plus `overscan` rows on each side. DOM node count stays O(viewport /
 * itemHeight) regardless of data size — a 10k-row list renders ~25 row nodes,
 * not 10k (asserted by the perf test). Absolute row positioning inside a
 * full-height spacer keeps the scrollbar native and scroll-jump-exact.
 *
 * `scrollTop` is clamped to the last item: a stale scroll offset (a dataset
 * that just shrank — filter applied — can leave the offset past the new end
 * for one frame) must never detach the window from the data.
 */
export function computeRange(count: number, itemHeight: number, scrollTop: number, viewportHeight: number, overscan: number): VirtualRange {
  const firstVisible = Math.min(Math.floor(Math.max(0, scrollTop) / itemHeight), Math.max(0, count - 1));
  const visibleCount = Math.ceil(Math.max(0, viewportHeight) / itemHeight);
  const startIndex = Math.max(0, firstVisible - overscan);
  const endIndex = Math.min(count - 1, firstVisible + visibleCount + overscan - 1);
  return { startIndex, endIndex, totalHeight: count * itemHeight };
}

export interface VirtualScrollTarget {
  index: number;
  /** Where the target row should land. @default "start" */
  align?: "start" | "center" | "end";
}
