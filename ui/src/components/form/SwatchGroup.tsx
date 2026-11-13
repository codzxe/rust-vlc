import { useId } from "react";
import type { ReactElement } from "react";
import type { AccentId } from "../../tokens/generated/tokens";
import { useRadiogroupKeyboard } from "./useRadiogroupKeyboard";
import "./form.css";

export interface SwatchOption {
  value: AccentId;
  /** Accessible name (caller-supplied — i18n at the screen). */
  label: string;
  /** Per-swatch disabled state (skipped by arrow-key roving). */
  disabled?: boolean;
}

export interface SwatchGroupProps {
  /** Group label — becomes the radiogroup's accessible name. */
  label: string;
  /** Selected accent id. */
  value: AccentId;
  /** Accent selection change — fires on click and on arrow-key moves. */
  onChange: (value: AccentId) => void;
  /** Swatch set — defaults to the 9 design accents in frame order. */
  options?: readonly SwatchOption[];
  /** Disabled group — every swatch inert. */
  disabled?: boolean;
  id?: string;
  className?: string;
}

/**
 * Swatch set order per Settings - global.svg (frame: orange first, then red,
 * yellow, green, teal, blue, purple, magenta, gray — the W26 reorder with
 * orange as the default accent). Labels default to the technical AccentId
 * (TokenSandbox precedent); screens pass localized labels via `options`.
 */
const DEFAULT_SWATCH_ORDER: readonly AccentId[] = [
  "orange",
  "red",
  "yellow",
  "green",
  "cyan",
  "blue",
  "violet",
  "pink",
  "gray",
];

const DEFAULT_SWATCHES: readonly SwatchOption[] = DEFAULT_SWATCH_ORDER.map((id) => ({ value: id, label: id }));

/**
 * SwatchGroup — the 9-accent picker from Settings - global.svg "Accent
 * color" (wave 26): rx6 surface track, 20×20 swatches; the selected swatch
 * sits in an accent-tinted rx4 slot (see form.css). Same radiogroup
 * semantics as RadioGroup/SegmentedControl: roving tabindex + arrow keys
 * with wrap-around; each swatch is a `role="radio"` button whose accessible
 * name is its localized label.
 */
export function SwatchGroup({
  label,
  value,
  onChange,
  options = DEFAULT_SWATCHES,
  disabled = false,
  id,
  className = "",
}: SwatchGroupProps): ReactElement {
  const autoId = useId();
  const groupId = id ?? autoId;
  // shared roving grammar (w26.k01 j01-2): four arrows, wrap-around,
  // disabled swatches skipped
  const onKeyDown = useRadiogroupKeyboard(options, value, onChange);

  return (
    <div
      role="radiogroup"
      id={groupId}
      aria-label={label}
      className={["lumen-swatchgroup", className].filter(Boolean).join(" ")}
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
            className="lumen-swatchgroup__swatch"
            data-accent={option.value}
            aria-checked={checked}
            aria-label={option.label}
            // roving tabindex: only the selected swatch is tabbable
            tabIndex={checked ? 0 : -1}
            disabled={disabled || option.disabled}
            onClick={() => {
              if (!checked) onChange(option.value);
            }}
          >
            <span className="lumen-swatchgroup__dot" aria-hidden="true" />
          </button>
        );
      })}
    </div>
  );
}
