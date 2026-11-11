import { useId } from "react";
import type { KeyboardEvent, ReactElement } from "react";
import "./form.css";

export interface SegmentedOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SegmentedControlProps {
  /** Group label — becomes the radiogroup's accessible name. */
  label: string;
  options: SegmentedOption[];
  /** Controlled selected value (matches an option value). */
  value: string;
  /** Selection change — fires on click and on arrow-key moves. */
  onChange: (value: string) => void;
  /** Disabled group — every segment inert. */
  disabled?: boolean;
  /** Error state flag. */
  invalid?: boolean;
  id?: string;
  className?: string;
}

/**
 * SegmentedControl — the Values.svg / Settings "Theme" control (wave 26):
 * rx6 surface track, rx4 segments; the active segment is accent-tinted with
 * an accent label, inactive segments are panel with tertiary labels (see
 * form.css). Semantics mirror RadioGroup (`role="radiogroup"`, native-button
 * `role="radio"` segments, roving tabindex, arrow keys with wrap-around) so
 * enum settings can swap RadioGroup → SegmentedControl without rewiring.
 */
export function SegmentedControl({
  label,
  options,
  value,
  onChange,
  disabled = false,
  invalid = false,
  id,
  className = "",
}: SegmentedControlProps): ReactElement {
  const autoId = useId();
  const groupId = id ?? autoId;

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    const vertical = event.key === "ArrowDown" || event.key === "ArrowUp";
    const horizontal = event.key === "ArrowRight" || event.key === "ArrowLeft";
    if (!vertical && !horizontal) return;
    const enabled = options.filter((option) => !option.disabled);
    if (enabled.length === 0) return;
    const currentIndex = enabled.findIndex((option) => option.value === value);
    const delta = event.key === "ArrowUp" || event.key === "ArrowLeft" ? -1 : 1;
    const next = enabled[(currentIndex + delta + enabled.length) % enabled.length];
    if (next && next.value !== value) onChange(next.value);
    event.preventDefault();
  };

  return (
    <div
      role="radiogroup"
      aria-label={label}
      aria-invalid={invalid || undefined}
      className={["lumen-segmented", className].filter(Boolean).join(" ")}
      onKeyDown={onKeyDown}
    >
      {options.map((option) => {
        const checked = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            id={`${groupId}-${option.value}`}
            className="lumen-segmented__segment"
            value={option.value}
            aria-checked={checked}
            // roving tabindex: only the selected segment is tabbable
            tabIndex={checked ? 0 : -1}
            disabled={disabled || option.disabled}
            onClick={() => {
              if (!checked) onChange(option.value);
            }}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
