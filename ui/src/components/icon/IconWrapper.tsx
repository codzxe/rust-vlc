import type { ReactNode } from "react";
import "./icon.css";

/**
 * IconWrapper — the framed icon container from the pinned design sheet
 * (design/figma/Icon Wrapper.svg, "Property 1=hover"): a bare icon at rest,
 * and on hover a subtle rounded backdrop (30×32 r4, white 6% in the frame)
 * that marks the glyph as an interactive control target.
 *
 * Sizes expose the hit areas the redesign needs (W26 contract: icon rail
 * targets ≥44px, control-bar buttons ≥30px, dense rows 24px):
 *   - `sm` 24×24  — dense rows (playlist/meta rows)
 *   - `md` 30×32  — the Icon Wrapper.svg hover frame itself (control bar)
 *   - `lg` 44×44  — icon rail / navigation targets
 *
 * `active` renders the orange accent state (W25 spec: active nav icons are
 * the ONE accent) via the same token chain as selected menu rows.
 *
 * The wrapper is presentational: wrap it in the interactive element (or use
 * IconButton) and give that element the accessible name.
 */

/** Hit-area presets (px). */
export type IconWrapperSize = "sm" | "md" | "lg";

export function IconWrapper({
  size = "md",
  active = false,
  className = "",
  children,
}: {
  /** Hit-area size. @default "md" (the 30×32 Icon Wrapper.svg frame) */
  size?: IconWrapperSize;
  /** Orange accent state (active nav icon / selected row). @default false */
  active?: boolean;
  className?: string;
  /** The icon glyph (usually an `<Icon />`). */
  children: ReactNode;
}) {
  return (
    <span
      className={[
        "lumen-icon-wrapper",
        `lumen-icon-wrapper--${size}`,
        active ? "lumen-icon-wrapper--active" : "",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </span>
  );
}
