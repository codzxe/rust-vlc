import { useId, forwardRef } from "react";
import type { InputHTMLAttributes } from "react";
import "./form.css";

export interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  /** Visible label. Required for a11y — pass aria-label via rest if hidden. */
  label: string;
  /** Error message; presence switches the field into the error state. */
  error?: string;
  /** Renders the label as the input's aria-label instead of a visible text
   * node (row grammar hosts that print their own label — w26.f02). The
   * reference mechanism for every field's hidden-label mode (w26.k01
   * j01-8: BrowseField/NumberField align to THIS — one text node). */
  hideLabel?: boolean;
}

/**
 * TextField primitive (wave 26 input grammar: #2a2a2a-class fill, 1px
 * #3a3a3a-class border, r6, focus orange — form.css token seams pending the
 * input tokens). Error state wires `aria-invalid` + `aria-describedby`
 * to the rendered error message (doc 08 §4.4/§5.4).
 */
export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(
  ({ label, error, hideLabel = false, id, className = "", ...rest }, ref) => {
    const autoId = useId();
    const inputId = id ?? autoId;
    const errorId = `${inputId}-error`;
    return (
      <div className={["lumen-field", className].filter(Boolean).join(" ")}>
        {hideLabel ? null : (
          <label className="lumen-field__label" htmlFor={inputId}>
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          type={rest.type ?? "text"}
          className="lumen-field__control"
          aria-label={hideLabel ? label : undefined}
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
