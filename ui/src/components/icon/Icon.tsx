import "./icon.css";
import { getIconGlyph, getTwoToneGlyph } from "./icons";
import type { IconName } from "./icons";

/**
 * Icon — 1.5-stroke line icon (doc 08 §1.4 icon system: stroke #8F8F8F →
 * --color-stroke-icon, stroke-width 1.5, ~48px hit wrapper per Icon
 * Wrapper.svg). Stroke is `currentColor` so hosts tint via tokens; state
 * variants (hover/active/disabled) are CSS classes in icon.css (§4.4).
 *
 * Emphasis strokes (issue #35, doc 08 §1.4 "2 and 3 for emphasis"; tokens
 * --stroke-icon-strong 2px / --stroke-icon-max 3px): the design's active-state
 * glyphs redraw at stroke-width 2 (e.g. Icon.svg "preferences, state=active"),
 * so `variant` picks the stroke weight. Colour stays host-driven via the
 * existing CSS state classes — variant only changes the stroke width.
 *
 * Decorative by default (`aria-hidden`); pass `title` for a named icon.
 *
 * w26.i16 TWO-TONE mode (menu frames: quiet #8F8F8F base strokes + #AB6314
 * detail elements — the sheet's `two-tone=yes` groups; h18 defect #3):
 * `twoTone` swaps in the glyph's two-tone face (a dedicated variant where
 * the frame adds geometry, else the shared entry) and adds
 * `.lumen-icon--two-tone`; icon.css then paints `lumen-icon__detail`
 * elements with `--color-accent-strong` (bright `--detail--bright` variants
 * with `--color-accent-icon`). Mono hosts are unchanged — the markers are
 * inert without the class.
 */

/** Stroke-weight variants, mapped 1:1 to the §1.4 stroke tokens. */
export type IconVariant = "default" | "strong" | "max";

const STROKE_WIDTHS: Record<IconVariant, number> = {
  default: 1.5, // --stroke-icon
  strong: 2, // --stroke-icon-strong
  max: 3, // --stroke-icon-max
};

export function Icon({
  name,
  size = 16,
  title,
  className = "",
  disabled = false,
  variant = "default",
  twoTone = false,
}: {
  name: IconName;
  /** Width/height in px (logical, DPI-independent — doc 08 §1.6). @default 16 */
  size?: number;
  /** Accessible name; when omitted the icon is aria-hidden. */
  title?: string;
  className?: string;
  /** Non-interactive disabled state (§4.4: 0.5 opacity + no pointer). */
  disabled?: boolean;
  /** Stroke-weight variant (§1.4 emphasis strokes). @default "default" */
  variant?: IconVariant;
  /** w26.i16 — render the two-tone face (menu iconography; see header). @default false */
  twoTone?: boolean;
}) {
  const glyph = twoTone ? getTwoToneGlyph(name) : getIconGlyph(name);
  const variantClass = variant === "default" ? "" : ` lumen-icon--${variant}`;
  const twoToneClass = twoTone ? " lumen-icon--two-tone" : "";
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={STROKE_WIDTHS[variant]}
      strokeLinecap="round"
      strokeLinejoin="round"
      role={title ? "img" : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      className={`lumen-icon${disabled ? " lumen-icon--disabled" : ""}${variantClass}${twoToneClass} ${className}`.trim()}
    >
      {title ? <title>{title}</title> : null}
      {glyph}
    </svg>
  );
}
