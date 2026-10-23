import { forwardRef, useId } from "react";
import type { InputHTMLAttributes } from "react";
import "./form.css";

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  /** Visible choice label. */
  label: string;
  /** Error state flag (doc 08 §4.4) — styles via tokens, caller wires aria-describedby. */
  invalid?: boolean;
}

/**
 * Checkbox primitive — native input (Space toggles, checked semantics free).
 * No checkbox export exists in the design package (doc 08 §1.5); styled from
 * the accent/icon language per doc 08 §2 interim rules.
 */
export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, invalid = false, disabled, id, className = "", ...rest }, ref) => {
    const autoId = useId();
    const inputId = id ?? autoId;
    return (
      <span className={["lumen-choice", "lumen-choice--checkbox", invalid && "lumen-choice--error", disabled && "lumen-choice--disabled", className]
        .filter(Boolean)
        .join(" ")}>
        <input
          ref={ref}
          id={inputId}
          type="checkbox"
          className="lumen-choice__input"
          aria-invalid={invalid || undefined}
          disabled={disabled}
          {...rest}
        />
        <label htmlFor={inputId}>{label}</label>
      </span>
    );
  },
);

Checkbox.displayName = "Checkbox";
