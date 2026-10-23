import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import "./button.css";

export interface ToggleProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onChange"> {
  /** Controlled on/off state. */
  checked: boolean;
  /** Change handler — fires on click AND keyboard (Space/Enter via native button). */
  onChange: (checked: boolean) => void;
}

/**
 * Toggle switch primitive (doc 08 §1.5 Toggle.svg, §4.5: "toggles → switch").
 * Exposes `role="switch"` + `aria-checked` — the doc's ARIA contract (a
 * `switch` carries checked state, not `aria-pressed`; deviation from the
 * task-card blurb recorded in states.md). Geometry per Toggle.svg: 68x32 rx6
 * track; ON: 33x26 accent-tinted fill + 26x26 rx3 accent-strong knob right;
 * OFF: surface-muted knob left.
 */
export const Toggle = forwardRef<HTMLButtonElement, ToggleProps>(
  ({ checked, onChange, disabled, type = "button", className = "", onClick, ...rest }, ref) => (
    <button
      ref={ref}
      type={type}
      role="switch"
      aria-checked={checked}
      className={["lumen-toggle", className].filter(Boolean).join(" ")}
      disabled={disabled}
      onClick={(event) => {
        if (disabled) return; // disabled means disabled — never fire
        onClick?.(event);
        onChange(!checked);
      }}
      {...rest}
    >
      <span className="lumen-toggle__fill" aria-hidden="true" />
      <span className="lumen-toggle__knob" aria-hidden="true" />
    </button>
  ),
);

Toggle.displayName = "Toggle";
