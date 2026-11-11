import { forwardRef } from "react";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import "./button.css";

/** Control hit areas per the W26 frames: 28 / 32 / 48 (`sm`/`md`/`lg` — the
 * `lg` wrapper is the ~48px Icon Wrapper hit target; all are ≥ 28px). */
export type IconButtonSize = "sm" | "md" | "lg";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /**
   * Accessible name (doc 08 §4.5: icon-only buttons require `aria-label`
   * from i18n keys). Required — callers pass the localized string.
   */
  label: string;
  /** Hit-area size. @default "md" (32px; `lg` = the ~48px Icon Wrapper) */
  size?: IconButtonSize;
  /** The icon glyph (SVG) — 16px line glyphs per the W26 icon frames. */
  children: ReactNode;
  /** Pending state — inert + `aria-busy` + spinner styling hook. */
  loading?: boolean;
}

/**
 * IconButton primitive (doc 08 §4/§1.5 icon system; W26 Figma restyle:
 * transparent rest + 1px transparent border, hover surface-hover chip per
 * Icon Wrapper.svg, 2px accent focus ring). Icon-only, so `label` is
 * mandatory for ARIA. Carries `lumen-icon-host` so nested `.lumen-icon`
 * glyphs pick up the icon system's state tints (icon.css).
 */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  (
    { label, size = "md", loading = false, disabled, type = "button", className = "", children, ...rest },
    ref,
  ) => (
    <button
      ref={ref}
      type={type}
      aria-label={label}
      aria-busy={loading || undefined}
      className={[
        "lumen-icon-button",
        "lumen-icon-host", // icon.css state protocol (hover/active glyph tints)
        size !== "md" && `lumen-icon-button--${size}`,
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      disabled={disabled || loading}
      {...rest}
    >
      {children}
    </button>
  ),
);

IconButton.displayName = "IconButton";
