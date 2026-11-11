// w26.b11 — frame-ported glyphs for the subtitle flow (per-frame
// pixel-perfect SVG ports, contract §4.4): the subtitle-file glyph
// ("subtitle one.svg" Icon — page + fold + two text lines, stroke 1.5
// #6B6B6B = --color-content-tertiary), the media-file glyph
// ("Add Subtitles to Playlist/Folder.svg" Frame 223 Icon — wider page with
// two diagonal ticks across the top edge, same stroke class) and the
// multi-candidate cycle glyph ("subtitle multiple.svg" Vectors 42/43 —
// stacked ←/→, stroke 1.5 #8F8F8F). Strokes are `currentColor` so hosts
// tint via tokens; geometry is redrawn on the frame coordinates (marked
// per glyph) — no raw colors live here.

import type { ReactElement } from "react";

export interface GlyphProps {
  /** Rendered width/height in px (glyphs are near-square; see each frame). */
  size?: number;
  className?: string;
}

/**
 * SubtitleFileGlyph — frame: subtitle one.svg "Icon" (x31–49, y28–48: an
 * 18×20 page, top-left fold cut, two inner text lines at y39/y42).
 */
export function SubtitleFileGlyph({ size = 19, className = "" }: GlyphProps): ReactElement {
  return (
    <svg
      viewBox="0 0 18 20"
      width={size}
      height={(size * 20) / 18}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      {/* Page outline — top edge starts at the fold (frame x37→rel 6). */}
      <path d="M6 0.5h8.5c1.8 0 3 1.3 3 3v13c0 1.8-1.2 3-3 3H3.5c-1.8 0-3-1.2-3-3V6" />
      {/* Fold flap — the crease from the top edge down to the left edge
       * (frame Rectangle 1: bar x36.25–37.75 y28–32 + rounded join). */}
      <path d="M6 0.5v3c0 1.4-1.1 2.5-2.5 2.5H.5" />
      {/* Inner text lines (frame Vector 9/10: x36–44 @y39, x36–40 @y42). */}
      <path d="M5 10.5h8M5 13.5h4" />
    </svg>
  );
}

/**
 * MediaFileGlyph — frame: Add Subtitles to Playlist/Folder.svg Frame 223
 * "Icon" (x33–51, y130–146: an 18×16 page, r2, top edge crossed by two
 * diagonal ticks at x37.95/x43.8 — the "filmstrip" mark that distinguishes
 * a media row from a subtitle row).
 */
export function MediaFileGlyph({ size = 19, className = "" }: GlyphProps): ReactElement {
  return (
    <svg
      viewBox="0 0 18 16"
      width={size}
      height={(size * 16) / 18}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      {/* Page outline (frame mask stroke0_1911_7545: 18×16, r2). */}
      <path d="M2 .5h14a1.5 1.5 0 0 1 1.5 1.5v12a1.5 1.5 0 0 1-1.5 1.5H2A1.5 1.5 0 0 1 .5 14V2A1.5 1.5 0 0 1 2 .5Z" />
      {/* Diagonal ticks across the top edge (frame Rectangle 16:
       * 37.95,130→39.05,134.78 and 43.8,130→44.67,134.75). */}
      <path d="M5.5.5 6.5 5M11 .5l.9 4.5" />
    </svg>
  );
}

/**
 * CycleGlyph — frame: subtitle multiple.svg Vectors 42/43 (x472–486,
 * y69–87: a stacked ←/→ pair, the multi-candidate hover affordance).
 */
export function CycleGlyph({ size = 18, className = "" }: GlyphProps): ReactElement {
  return (
    <svg
      viewBox="0 0 14 18"
      width={(size * 14) / 18}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      {/* Left arrow (frame Vector 43). */}
      <path d="M13 3.5H0M3.5 7 0 3.5 3.5 0" />
      {/* Right arrow (frame Vector 42). */}
      <path d="M1 14.5h13M10.5 11l3.5 3.5-3.5 3.5" />
    </svg>
  );
}
