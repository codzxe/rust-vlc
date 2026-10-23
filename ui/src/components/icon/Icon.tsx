import "./icon.css";
import { getIconGlyph } from "./icons";
import type { IconName } from "./icons";

/**
 * Icon — 1.5-stroke line icon (doc 08 §1.4 icon system: stroke #8F8F8F →
 * --color-stroke-icon, stroke-width 1.5, ~48px hit wrapper per Icon
 * Wrapper.svg). Stroke is `currentColor` so hosts tint via tokens; state
 * variants (hover/active/disabled) are CSS classes in icon.css (§4.4).
 *
 * Decorative by default (`aria-hidden`); pass `title` for a named icon.
 */
export function Icon({
  name,
  size = 16,
  title,
  className = "",
  disabled = false,
}: {
  name: IconName;
  /** Width/height in px (logical, DPI-independent — doc 08 §1.6). @default 16 */
  size?: number;
  /** Accessible name; when omitted the icon is aria-hidden. */
  title?: string;
  className?: string;
  /** Non-interactive disabled state (§4.4: 0.5 opacity + no pointer). */
  disabled?: boolean;
}) {
  const glyph = getIconGlyph(name);
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
      role={title ? "img" : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      className={`lumen-icon${disabled ? " lumen-icon--disabled" : ""} ${className}`.trim()}
    >
      {title ? <title>{title}</title> : null}
      {glyph}
    </svg>
  );
}
