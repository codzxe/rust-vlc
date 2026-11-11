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
 * top-right of the dialog title bar). Geometry: the frame's 10×10 glyph on
 * the shared 24-unit grid — 15 units wide (renders 10px at size 16) with a
 * 2.25-unit stroke (renders the frame's 1.5px); currentColor keeps the
 * quiet token ink.
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
      strokeWidth={2.25}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {/* frame: Open network stream.svg (Vector 20/21 — 10×10 ✕, 1.5 stroke;
       * 15 units of 24 = 10px rendered at size 16, w26.a18 pixel re-verify) */}
      <path d="M4.5 4.5L19.5 19.5" />
      <path d="M19.5 4.5L4.5 19.5" />
    </svg>
  );
}

/**
 * Chevron — frame: "Audio - Equalizer.svg" ("Icon"/"Vector 27": right-pointing
 * 1.5-stroke chevron, #6B6B6B ink, right end of the card's toggle bar; the
 * expanded state points down — hosts rotate via CSS on aria-expanded).
 * Geometry: ~10×6 frame glyph on the shared 24-unit grid (14 units wide
 * renders ≈9px at size 16, stroke 2.25 units = the frame's 1.5px).
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
      strokeWidth={2.25}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {/* frame: Audio - Equalizer.svg (Vector 27 — right chevron, 1.5 stroke) */}
      <path d="M8 8.5L16.5 12L8 15.5" />
    </svg>
  );
}
