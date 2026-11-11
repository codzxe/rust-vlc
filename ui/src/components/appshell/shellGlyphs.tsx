// shellGlyphs — the W26 shell-chrome glyphs that the pinned icon registry
// (components/icon/icons.tsx) does not carry yet: window controls (min /
// max / close, Player.svg topbar), the sidebar-header controls (back
// chevron, ellipsis, search, sort, list/grid view toggles — sidebar.svg)
// and the small plus used by the rail's pinned stop and the create row.
//
// Geometry follows the icon system's conventions (doc 08 §1.4): 24×24
// viewBox, stroke currentColor at 1.5 (2 for the window controls' heavier
// frame glyphs), round caps/joins, ink from the host's `color` — so every
// state tint stays token-driven. All glyphs are DECORATIVE (aria-hidden):
// their buttons carry i18n'd aria-labels (doc 08 §4.5).
//
// [PROPOSAL] promote these into the registry (a follow-up icon-lane card)
// once the frame names are pinned; until then they live with their only
// consumer (appshell).

import type { ReactNode } from "react";

/** Shared stroked-glyph shell — same shape contract as `<Icon>`. */
function Glyph({
  size = 16,
  strokeWidth = 1.5,
  filled = false,
  children,
}: {
  size?: number;
  strokeWidth?: number;
  /** Fill-driven glyphs (dots) pin fill=currentColor + stroke=none. */
  filled?: boolean;
  children: ReactNode;
}) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill={filled ? "currentColor" : "none"}
      stroke={filled ? "none" : "currentColor"}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className="lumen-icon"
    >
      {children}
    </svg>
  );
}

/** Window minimize (Player.svg topbar: a single dash, 2px stroke). */
export function GlyphMinimize({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size} strokeWidth={2}>
      <path d="M5 12H19" />
    </Glyph>
  );
}

/** Window maximize (hollow square, 2px stroke). */
export function GlyphMaximize({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size} strokeWidth={2}>
      <rect x="6" y="6" width="12" height="12" rx="1.5" />
    </Glyph>
  );
}

/** Window close (×, 2px stroke). */
export function GlyphClose({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size} strokeWidth={2}>
      <path d="M6 6L18 18" />
      <path d="M18 6L6 18" />
    </Glyph>
  );
}

/** Plus (rail pinned stop + create-playlist row; Player.svg rail foot). */
export function GlyphPlus({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size} strokeWidth={2}>
      <path d="M12 5V19" />
      <path d="M5 12H19" />
    </Glyph>
  );
}

/** Back chevron (sidebar.svg header: left-pointing chevron). */
export function GlyphChevronLeft({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size}>
      <path d="M14 6L8 12L14 18" />
    </Glyph>
  );
}

/** Overflow ellipsis (sidebar.svg header right cluster). */
export function GlyphMore({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size} filled>
      <circle cx="5.5" cy="12" r="1.6" />
      <circle cx="12" cy="12" r="1.6" />
      <circle cx="18.5" cy="12" r="1.6" />
    </Glyph>
  );
}

/** Search (sidebar.svg header right cluster: magnifier). */
export function GlyphSearch({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size}>
      <circle cx="11" cy="11" r="6" />
      <path d="M15.5 15.5L20 20" />
    </Glyph>
  );
}

/** Sort (sidebar.svg meta row: descending bars + direction arrow). */
export function GlyphSort({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size}>
      <path d="M7 5V17" />
      <path d="M4 14L7 17L10 14" />
      <path d="M13 7H19" />
      <path d="M13 12H17" />
      <path d="M13 17H15" />
    </Glyph>
  );
}

/** List view (sidebar.svg meta row: dotted rows). */
export function GlyphList({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size}>
      <path d="M9 6H19" />
      <path d="M9 12H19" />
      <path d="M9 18H19" />
      <path d="M5 6H5.01" strokeWidth={2} />
      <path d="M5 12H5.01" strokeWidth={2} />
      <path d="M5 18H5.01" strokeWidth={2} />
    </Glyph>
  );
}

/** Grid view (sidebar.svg meta row: 2×2 tiles). */
export function GlyphGrid({ size = 16 }: { size?: number }) {
  return (
    <Glyph size={size}>
      <rect x="4" y="4" width="7" height="7" rx="1" />
      <rect x="13" y="4" width="7" height="7" rx="1" />
      <rect x="4" y="13" width="7" height="7" rx="1" />
      <rect x="13" y="13" width="7" height="7" rx="1" />
    </Glyph>
  );
}

/** Drag grip (playlist rows, Player.png sidebar: two dotted columns). */
export function GlyphGrip({ size = 14 }: { size?: number }) {
  return (
    <Glyph size={size} filled>
      <circle cx="9" cy="6" r="1.3" />
      <circle cx="15" cy="6" r="1.3" />
      <circle cx="9" cy="12" r="1.3" />
      <circle cx="15" cy="12" r="1.3" />
      <circle cx="9" cy="18" r="1.3" />
      <circle cx="15" cy="18" r="1.3" />
    </Glyph>
  );
}
