import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import "./form.css";

export interface SwitchProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onChange"> {
  /** Controlled on/off state. */
  checked: boolean;
  /** Change handler — fires on click AND keyboard (Space/Enter via native button). */
  onChange: (checked: boolean) => void;
}

/**
 * Switch — the W26 Toggle.svg control, owned by the form family (wave 26:
 * the settings rows consume it through this module; the button family keeps
 * its own Toggle for the control-parts lane). Same public contract as the
 * button-family Toggle it replaces here: `role="switch"` + `aria-checked`,
 * onChange on click/keyboard, native disabled gating.
 *
 * Geometry is an honest port of Toggle.svg (see form.css): 68×32 rx6 track;
 * ON = accent-tinted text segment ("ON" in accent) + accent-strong fill
 * right; OFF = stroke-border fill left + panel text segment ("OFF"
 * tertiary). The ON/OFF micro labels are decorative (aria-hidden) — the
 * switch's checked state is conveyed by aria-checked.
 */
export const Switch = forwardRef<HTMLButtonElement, SwitchProps>(
  ({ checked, onChange, disabled, type = "button", className = "", onClick, ...rest }, ref) => (
    <button
      ref={ref}
      type={type}
      role="switch"
      aria-checked={checked}
      className={["lumen-switch", className].filter(Boolean).join(" ")}
      disabled={disabled}
      onClick={(event) => {
        if (disabled) return; // disabled means disabled — never fire
        onClick?.(event);
        onChange(!checked);
      }}
      {...rest}
    >
      <span className="lumen-switch__text lumen-switch__text--on" aria-hidden="true">
        ON
      </span>
      <span className="lumen-switch__fill" aria-hidden="true" />
      <span className="lumen-switch__text lumen-switch__text--off" aria-hidden="true">
        OFF
      </span>
    </button>
  ),
);

Switch.displayName = "Switch";
