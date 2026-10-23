import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import "./button.css";

/** Button variants grounded in doc 08 (don't invent):
 * - `default` — Button.svg: neutral surface-deep fill, rx4.
 * - `primary` — onboarding primary button: accent fill, rx6 (§1.4). */
export type ButtonVariant = "default" | "primary";

/** Control heights from doc 08 §1.4: 28 / 32 / 48 (`sm` / `md` / `lg`). */
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

/**
 * Button primitive (doc 08 §4/§1.5). Semantic tokens only (§3.2); all
 * interactive states (hover/pressed/focus-visible/disabled/loading) styled
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
        variant !== "default" && `lumen-button--${variant}`,
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
