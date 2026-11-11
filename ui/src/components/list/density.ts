/**
 * Row density variants + Figma-measured pitches (W25 spec §2), split from
 * Row.tsx so the component file stays fast-refresh-clean (react-refresh
 * only-export-components; same pattern as menu/timings.ts).
 */

/**
 * Row density variants (W25 spec §2 row grammar):
 * - "comfortable" — playlist / media rows, ~66px, r10 (playlist.svg, media.svg)
 * - "compact" — menu & settings rows, 36–44px (menu.svg, Settings - global.svg)
 */
export type RowDensity = "comfortable" | "compact";

/**
 * Figma-measured row pitches per density (structural px — no row-height
 * token exists). Pass to <VirtualList itemHeight={ROW_HEIGHTS[density]}> so
 * the windowing math and the rendered row stay in lock-step.
 */
export const ROW_HEIGHTS: Record<RowDensity, number> = {
  comfortable: 66, /* frame: playlist.svg / media.svg — 32px thumb + insets */
  compact: 40, /* frame: menu.svg + Settings - global.svg — 36–44px rows */
};
