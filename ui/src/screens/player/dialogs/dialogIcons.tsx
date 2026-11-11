// Player-dialog glyphs (w26.a18) — the two small stroke glyphs the dialog
// grammar needs that the shared icon set (components/icon) does not carry
// yet: the header close ✕ and the tool-card chevron. Geometry is mined from
// the W25 frames (paths translated onto the shared 24×24 grid the Icon shell
// uses, stroke via currentColor so tokens tint them — same conventions as
// components/icon/icons.tsx). Additive local module: adding the glyphs to
// the shared icon set is the Lead's follow-up (recorded in the lane log).
import type { SVGProps } from "react";

/**
 * Close ✕ — frame: "Open network stream.svg" / "Keyboard Shortcuts.svg"
 * ("Icon Wrapper"/"Icon": two crossed 1.5-stroke lines, quiet #8F8F8F ink,
 * top-right of the dialog title bar). currentColor keeps the quiet token ink.
 */
export function CloseGlyph({
  size = 16,
  ...rest
}: { size?: number } & Omit<SVGProps<SVGSVGElement>, "width" | "height">) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {/* frame: Open network stream.svg (Vector 20/21 — 10×10 ✕, 1.5 stroke) */}
      <path d="M8.5 8.5L15.5 15.5" />
      <path d="M15.5 8.5L8.5 15.5" />
    </svg>
  );
}

/**
 * Chevron — frame: "Audio - Equalizer.svg" ("Icon"/"Vector 27": right-pointing
 * 1.5-stroke chevron, #6B6B6B ink, right end of the card's toggle bar; the
 * expanded state points down — hosts rotate via CSS on aria-expanded).
 */
export function ChevronGlyph({
  size = 16,
  ...rest
}: { size?: number } & Omit<SVGProps<SVGSVGElement>, "width" | "height">) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {/* frame: Audio - Equalizer.svg (Vector 27 — right chevron, 1.5 stroke) */}
      <path d="M9.5 6.5L15.5 12L9.5 17.5" />
    </svg>
  );
}
