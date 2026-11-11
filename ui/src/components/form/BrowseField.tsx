import { forwardRef, useId } from "react";
import type { InputHTMLAttributes } from "react";
import "./form.css";

export interface BrowseFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  /** Visible label. Required for a11y — pass aria-label via rest if hidden. */
  label: string;
  /** Caption for the trailing action button (caller-supplied — i18n at the screen). */
  actionLabel: string;
  /** Trailing action — opens the file picker (screen-level concern). */
  onAction?: () => void;
  /** Error message; presence switches the field into the error state. */
  error?: string;
  /** Renders the label but hides it visually (keeps the accessible name). */
  hideLabel?: boolean;
}

/**
 * BrowseField — the settings path input + action combo (wave 26; frame:
 * Settings - interface.svg "Skin file" / Settings - global.svg "Log file"):
 * the standard TextField control with a 32px r6 panel button (min-width 100)
 * beside it. The action is caller-wired (`onAction`) so the component stays
 * transport-agnostic; the input keeps the full TextField contract.
 */
export const BrowseField = forwardRef<HTMLInputElement, BrowseFieldProps>(
  ({ label, actionLabel, onAction, error, hideLabel = false, id, className = "", disabled, ...rest }, ref) => {
    const autoId = useId();
    const inputId = id ?? autoId;
    const errorId = `${inputId}-error`;
    return (
      <div className={["lumen-field", className].filter(Boolean).join(" ")}>
        <label
          className={["lumen-field__label", hideLabel ? "lumen-field__label--hidden" : ""]
            .filter(Boolean)
            .join(" ")}
          htmlFor={inputId}
        >
          {label}
        </label>
        <div className="lumen-browse">
          <input
            ref={ref}
            id={inputId}
            type={rest.type ?? "text"}
            className="lumen-field__control lumen-browse__control"
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? errorId : undefined}
            disabled={disabled}
            {...rest}
          />
          <button
            type="button"
            className="lumen-browse__action"
            onClick={() => {
              if (!disabled) onAction?.();
            }}
            disabled={disabled}
          >
            {actionLabel}
          </button>
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

BrowseField.displayName = "BrowseField";
