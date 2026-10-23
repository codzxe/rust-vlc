import { useId, forwardRef } from "react";
import type { InputHTMLAttributes } from "react";
import "./form.css";

export interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  /** Visible label. Required for a11y — pass aria-label via rest if hidden. */
  label: string;
  /** Error message; presence switches the field into the error state. */
  error?: string;
  /** Renders the label but hides it visually (keeps the accessible name). */
  hideLabel?: boolean;
}

/**
 * TextField primitive (doc 08 §1.4 onboarding text input: surface-deep fill,
 * 2px surface stroke). Error state wires `aria-invalid` + `aria-describedby`
 * to the rendered error message (doc 08 §4.4/§5.4).
 */
export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(
  ({ label, error, hideLabel = false, id, className = "", ...rest }, ref) => {
    const autoId = useId();
    const inputId = id ?? autoId;
    const errorId = `${inputId}-error`;
    return (
      <div className={["lumen-field", className].filter(Boolean).join(" ")}>
        <label className="lumen-field__label" htmlFor={inputId} style={hideLabel ? visuallyHidden : undefined}>
          {label}
        </label>
        <input
          ref={ref}
          id={inputId}
          type={rest.type ?? "text"}
          className="lumen-field__control"
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : undefined}
          {...rest}
        />
        {error ? (
          <p className="lumen-field__error" id={errorId}>
            {error}
          </p>
        ) : null}
      </div>
    );
  },
);

TextField.displayName = "TextField";

const visuallyHidden = {
  position: "absolute",
  width: 1,
  height: 1,
  padding: 0,
  margin: -1,
  overflow: "hidden",
  clip: "rect(0 0 0 0)",
  whiteSpace: "nowrap",
  border: 0,
} as const;
