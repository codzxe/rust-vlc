import { forwardRef } from "react";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import "./button.css";

/** Control heights from doc 08 §1.4/§1.5: 28 / 32 / 48 hit wrapper (`sm`/`md`/`lg`). */
export type IconButtonSize = "sm" | "md" | "lg";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /**
   * Accessible name (doc 08 §4.5: icon-only buttons require `aria-label`
   * from i18n keys). Required — callers pass the localized string.
   */
  label: string;
  /** Hit-area size. @default "md" (32px; `lg` = the ~48px Icon Wrapper) */
  size?: IconButtonSize;
  /** The icon glyph (SVG). */
  children: ReactNode;
  /** Pending state — inert + `aria-busy` + spinner styling hook. */
  loading?: boolean;
}

/**
 * IconButton primitive (doc 08 §4/§1.5 icon system: 1.5px stroke icons on a
 * ~48px hit wrapper). Icon-only, so `label` is mandatory for ARIA.
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
