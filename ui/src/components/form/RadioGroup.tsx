import { useId } from "react";
import type { ReactElement } from "react";
import { useRadiogroupKeyboard } from "./useRadiogroupKeyboard";
import "./form.css";

export interface RadioOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface RadioGroupProps {
  /** Group label — becomes the radiogroup's accessible name (doc 08 §4.5). */
  label: string;
  options: RadioOption[];
  /** Controlled selected value (matches an option value). */
  value: string;
  /** Selection change — fires on click and on arrow-key moves. */
  onChange: (value: string) => void;
  /** Disabled group — every option inert. */
  disabled?: boolean;
  /** Error state flag (doc 08 §4.4). */
  invalid?: boolean;
  id?: string;
  className?: string;
}

/**
 * RadioGroup primitive — `role="radiogroup"` with roving tabindex and
 * arrow-key navigation (doc 08 §4.5 focus rules). Options are native
 * `type="radio"` inputs sharing one generated name, so checked semantics,
 * Space activation and form integration come from the platform.
 */
export function RadioGroup({
  label,
  options,
  value,
  onChange,
  disabled = false,
  invalid = false,
  id,
  className = "",
}: RadioGroupProps): ReactElement {
  const autoId = useId();
  const groupId = id ?? autoId;
  const name = `${groupId}-radio`;
  // shared roving grammar (w26.k01 j01-2): four arrows, wrap-around,
  // disabled options skipped
  const onKeyDown = useRadiogroupKeyboard(options, value, onChange);

  return (
    <fieldset
      role="radiogroup"
      id={groupId}
      aria-label={label}
      aria-invalid={invalid || undefined}
      className={["lumen-radiogroup", invalid && "lumen-choice--error", className].filter(Boolean).join(" ")}
      onKeyDown={onKeyDown}
      disabled={disabled}
    >
      <div className="lumen-radiogroup__options">
        {options.map((option) => {
          const checked = option.value === value;
          const optionId = `${groupId}-${option.value}`;
          return (
            <span
              key={option.value}
              className={[
                "lumen-choice",
                "lumen-choice--radio", /* Values-1.svg row grammar (form.css) */
                option.disabled && "lumen-choice--disabled",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              <input
                type="radio"
                className="lumen-choice__input"
                name={name}
                id={optionId}
                value={option.value}
                checked={checked}
                // roving tabindex (doc 08 §4.5): only the selected option is tabbable
                tabIndex={checked ? 0 : -1}
                disabled={disabled || option.disabled}
                onChange={() => {
                  if (!checked) onChange(option.value);
                }}
              />
              <label htmlFor={optionId}>{option.label}</label>
            </span>
          );
        })}
      </div>
    </fieldset>
  );
}
