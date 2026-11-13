import { forwardRef, useId } from "react";
import type { InputHTMLAttributes } from "react";
import "./form.css";

export interface NumberFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size" | "type"> {
  /** Visible label (unless `labelHidden` — then the accessible name only). */
  label: string;
  /** Render the label as the input's aria-label instead of a visible text
   * node (row grammar hosts that print their own label — w26.f02). */
  labelHidden?: boolean;
  /** Error message; presence switches the field into the error state. */
  error?: string;
  min?: number;
  max?: number;
  /** Stepper step (doc 08 §1.5 Values.svg steppers). */
  step?: number;
  /** Standard change handler — fired by typing AND by the pointer steppers.
   * Stepper events are SYNTHETIC (jsdom has no stepUp/stepDown): `target` /
   * `currentTarget` carry `value` (string) + `valueAsNumber`, but no event
   * metadata (`name` is absent, `isTrusted` is not set) — read the value,
   * not the event (honest-gap note, form/states.md; w26.k01 j01-11). */
  onChange?: React.ChangeEventHandler<HTMLInputElement>;
}

/**
 * NumberField primitive — text input + stepper affordance (doc 08 §1.5
 * Values.svg value steppers). Steppers clamp to min/max and report through
 * the standard onChange; steppers disable at the bounds (no fabricated
 * out-of-range values). min/max/step are forwarded to the native input
 * (w26.k01 j01-1), so keyboard users get the same contract pointer users
 * do: native arrows respect the bounds and the step, instead of the mouse
 * clamping while the keyboard stepped freely past the limits. Keyboard:
 * native number input (arrows, Home/End).
 */
export const NumberField = forwardRef<HTMLInputElement, NumberFieldProps>(
  ({ label, labelHidden = false, error, min, max, step = 1, id, className = "", onChange, disabled, value, ...rest }, ref) => {
    const autoId = useId();
    const inputId = id ?? autoId;
    const errorId = `${inputId}-error`;
    // `value` is string | number (InputHTMLAttributes): numeric strings step
    // too (j01-11 — a controlled "7" used to make the steppers silently
    // no-op); empty or non-numeric text still means "no value → no fabricated
    // step".
    const parsed =
      typeof value === "number" ? value : typeof value === "string" && value.trim() !== "" ? Number(value) : Number.NaN;
    const numeric = Number.isFinite(parsed) ? parsed : undefined;

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
        {labelHidden ? null : (
          <label className="lumen-field__label" htmlFor={inputId}>
            {label}
          </label>
        )}
        <div className="lumen-number">
          <input
            ref={ref}
            id={inputId}
            className="lumen-field__control lumen-number__control"
            type="number"
            inputMode="numeric"
            aria-label={labelHidden ? label : undefined}
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? errorId : undefined}
            value={value}
            disabled={disabled}
            min={min}
            max={max}
            step={step}
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
