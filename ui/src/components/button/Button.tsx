import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import "./button.css";

/** Button variants grounded in the W26 Figma frames (w26.a03 lane log):
 * - `default` — the secondary workhorse (dialog "Cancel"/"Browse"): raised
 *   fill + 1px border, r6, hover surface-hover. Frames: 100x32 r6, fill
 *   #1B1B1B, label #8F8F8F (Open network stream / Keyboard Shortcuts /
 *   new playlist / Player - customize).
 * - `secondary` — explicit alias of `default` (renders the same classes).
 * - `primary` — onboarding CTA / play pill: accent fill, on-accent (dark)
 *   glyph, r6 (Onboarding.svg "Next" 100x32 r6 #D37918 + #161616; play.svg
 *   pill 54x28 r6). No frame shows a pill-radius (r30) button — r30 art is
 *   onboarding accent-swatch rings only — so every variant is r6.
 * - `tinted` — dialog confirm action ("Save"): accent-tinted fill, accent
 *   label (Open network stream.svg / Keyboard Shortcuts.svg / new
 *   playlist.svg / Player - customize.svg: #371F06 bg + #D37918 text).
 * - `ghost` — transparent, 1px transparent border, hover surface-hover
 *   (Icon Wrapper.svg hover chip). */
export type ButtonVariant = "default" | "primary" | "secondary" | "ghost" | "tinted";

/** Control heights per the W26 frames: 28 (play pill/compact rows) / 32
 * (dialog buttons) / 36 (segmented + footer bars) — `sm` / `md` / `lg`. */
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual variant. @default "default" */
  variant?: ButtonVariant;
  /** Control height. @default "md" */
  size?: ButtonSize;
  /**
   * Pending state (doc 08 §4.4/§5): 0.7 opacity + CSS spinner + `aria-busy`.
   * A loading button never fires click handlers — it is fully inert.
   */
  loading?: boolean;
}

/** Variants that render without a modifier class (base = secondary look). */
const BASE_VARIANTS: readonly ButtonVariant[] = ["default", "secondary"];

/**
 * Button primitive (doc 08 §4/§1.5; W26 Figma restyle — anatomy in
 * `button.css` + docs/waves/w26/logs/w26.a03.md). Semantic tokens only (§3.2);
 * all interactive states (hover/pressed/focus-visible/disabled/loading) styled
 * via tokens in `button.css`. Native `<button>` gives Enter/Space for free.
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    { variant = "default", size = "md", loading = false, disabled, type = "button", className = "", ...rest },
    ref,
  ) => (
    <button
      ref={ref}
      type={type}
      className={[
        "lumen-button",
        !BASE_VARIANTS.includes(variant) && `lumen-button--${variant}`,
        size !== "md" && `lumen-button--${size}`,
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      aria-busy={loading || undefined}
      disabled={disabled || loading}
      {...rest}
    />
  ),
);

Button.displayName = "Button";
