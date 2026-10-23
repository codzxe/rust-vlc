import { forwardRef, useId } from "react";
import type { InputHTMLAttributes } from "react";
import "./form.css";

export interface NumberFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size" | "type"> {
  /** Visible label. */
  label: string;
  /** Error message; presence switches the field into the error state. */
  error?: string;
  min?: number;
  max?: number;
  /** Stepper step (doc 08 §1.5 Values.svg steppers). */
  step?: number;
}

/**
 * NumberField primitive — text input + stepper affordance (doc 08 §1.5
 * Values.svg value steppers). Steppers clamp to min/max and report through
 * the standard onChange; steppers disable at the bounds (no fabricated
 * out-of-range values). Keyboard: native number input (arrows, Home/End).
 */
export const NumberField = forwardRef<HTMLInputElement, NumberFieldProps>(
  ({ label, error, min, max, step = 1, id, className = "", onChange, disabled, value, ...rest }, ref) => {
    const autoId = useId();
    const inputId = id ?? autoId;
    const errorId = `${inputId}-error`;
    const numeric = typeof value === "number" ? value : undefined;

    const clamp = (next: number): number => {
      let out = next;
      if (typeof min === "number") out = Math.max(min, out);
      if (typeof max === "number") out = Math.min(max, out);
      return out;
    };

    const stepBy = (direction: 1 | -1) => (): void => {
      if (disabled || numeric === undefined) return; // no value -> no fabricated step
      const next = clamp(numeric + step * direction);
      if (next === numeric) return;
      onChange?.({
        target: { value: String(next), valueAsNumber: next },
        currentTarget: { value: String(next), valueAsNumber: next },
      } as unknown as React.ChangeEvent<HTMLInputElement>);
    };

    const atMin = typeof min === "number" && numeric !== undefined && numeric <= min;
    const atMax = typeof max === "number" && numeric !== undefined && numeric >= max;

    return (
      <div className={["lumen-field", className].filter(Boolean).join(" ")}>
        <label className="lumen-field__label" htmlFor={inputId}>
          {label}
        </label>
        <div className="lumen-number">
          <input
            ref={ref}
            id={inputId}
            className="lumen-field__control lumen-number__control"
            type="number"
            inputMode="numeric"
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? errorId : undefined}
            value={value}
            disabled={disabled}
            onChange={onChange}
            {...rest}
          />
          <span className="lumen-number__steppers">
            <button
              type="button"
              className="lumen-number__stepper"
              data-testid="stepper-up"
              tabIndex={-1} // stepper is a pointer affordance; keyboard uses the input's own arrows
              aria-hidden="true"
              disabled={disabled || atMax}
              onClick={stepBy(1)}
            >
              <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
                <path d="M5 1v8M1 5h8" stroke="currentColor" strokeWidth="1.5" />
              </svg>
            </button>
            <button
              type="button"
              className="lumen-number__stepper"
              data-testid="stepper-down"
              tabIndex={-1}
              aria-hidden="true"
              disabled={disabled || atMin}
              onClick={stepBy(-1)}
            >
              <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
                <path d="M1 5h8" stroke="currentColor" strokeWidth="1.5" />
              </svg>
            </button>
          </span>
        </div>
        {error ? (
          <p className="lumen-field__error" id={errorId}>
            {error}
          </p>
        ) : null}
      </div>
    );
  },
);

NumberField.displayName = "NumberField";
