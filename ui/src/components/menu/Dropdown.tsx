import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement, Ref } from "react";
import { Icon } from "../icon/Icon";
import type { IconName } from "../icon/icons";
import { identityResolveLabel } from "./types";
import type { MenuDensity, ResolveLabel } from "./types";
import { TYPEAHEAD_RESET_MS } from "./timings";
import "./menu.css";

/**
 * Dropdown / Select (W26 frames `Dropdown.svg` + `Setting Preset.svg`:
 * trigger rx6 with border + chevron; open list rx6 `#191919` with a subtle
 * drop shadow; option rows rx6, hover/selected `--color-surface-selected`
 * with an orange check on the selected row) — combobox trigger with
 * `aria-haspopup="listbox"`, popup `role="listbox"` with roving tabindex
 * options, arrows/Home/End, Enter/Space select, Escape/Tab close with focus
 * restoration to the trigger, typeahead jump, disabled options skipped and
 * not selectable. Portal-free absolute positioning inside a
 * relatively-positioned wrapper.
 *
 * Density (W26): `comfortable` = `Dropdown.svg` (48px trigger/rows);
 * `compact` = `Setting Preset.svg` (~40px) for settings rows.
 */
export interface DropdownProps<K extends string = string> {
  options: DropdownOption<K>[];
  /** Controlled selected value (matches an option value). */
  value: string;
  onChange: (value: string) => void;
  /** Accessible name for the trigger (already-resolved string, RadioGroup pattern). */
  label: string;
  /**
   * i18n seam: resolves option `labelKey`s (wire `useMsg()`, doc 05 §8.3).
   * @default identity
   */
  resolveLabel?: ResolveLabel<K>;
  disabled?: boolean;
  invalid?: boolean;
  /** Row scale (W26): compact ≈ `Setting Preset.svg` settings dropdowns. @default "comfortable" */
  density?: MenuDensity;
  id?: string;
  className?: string;
  /** React 19 ref-as-prop on the trigger button. */
  ref?: Ref<HTMLButtonElement>;
}

export interface DropdownOption<K extends string = string> {
  value: string;
  labelKey: K;
  /** Leading line icon (W26 `Setting Preset.svg` options). */
  icon?: IconName;
  disabled?: boolean;
}

export function Dropdown<K extends string>({
  options,
  value,
  onChange,
  label,
  resolveLabel = identityResolveLabel,
  disabled = false,
  invalid = false,
  density = "comfortable",
  id,
  className = "",
  ref,
}: DropdownProps<K>): ReactElement {
  const autoId = useId();
  const rootId = id ?? autoId;
  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const optionRefs = useRef<Array<HTMLDivElement | null>>([]);
  const typeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [typed, setTyped] = useState("");

  useEffect(
    () => () => {
      if (typeTimer.current !== null) clearTimeout(typeTimer.current);
    },
    [],
  );

  // Outside pointer dismiss.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent): void => {
      if (rootRef.current !== null && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const enabledIndex = (from: number, delta: number): number => {
    if (options.length === 0) return -1;
    let index = from;
    for (let step = 0; step < options.length; step += 1) {
      index = (index + delta + options.length) % options.length;
      const option = options[index];
      if (option !== undefined && option.disabled !== true) return index;
    }
    return -1;
  };

  const focusIndex = (index: number): void => {
    setActiveIndex(index);
    optionRefs.current[index]?.focus();
  };

  const closeAndRestore = (): void => {
    setOpen(false);
    setTyped("");
    triggerRef.current?.focus();
  };

  const select = (index: number): void => {
    const option = options[index];
    if (option === undefined || option.disabled === true) return;
    onChange(option.value);
    closeAndRestore();
  };

  const openList = (): void => {
    const selectedIndex = options.findIndex((option) => option.value === value);
    const selectedOption = selectedIndex >= 0 ? options[selectedIndex] : undefined;
    const initial =
      selectedOption !== undefined && selectedOption.disabled !== true
        ? selectedIndex
        : enabledIndex(-1, 1);
    setOpen(true);
    setActiveIndex(initial);
  };

  // Focus the initial option when the list opens (WAI-ARIA listbox).
  useEffect(() => {
    if (!open) return;
    if (activeIndex >= 0) optionRefs.current[activeIndex]?.focus();
    // Open-transition only — later activeIndex changes focus themselves.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const handleListKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const { key } = event;
    if (key === "ArrowDown" || key === "ArrowUp") {
      const base = activeIndex >= 0 ? activeIndex : key === "ArrowDown" ? -1 : 0;
      const next = enabledIndex(base, key === "ArrowDown" ? 1 : -1);
      if (next >= 0) focusIndex(next);
      event.preventDefault();
      return;
    }
    if (key === "Home") {
      const first = enabledIndex(-1, 1);
      if (first >= 0) focusIndex(first);
      event.preventDefault();
      return;
    }
    if (key === "End") {
      const last = enabledIndex(options.length, -1);
      if (last >= 0) focusIndex(last);
      event.preventDefault();
      return;
    }
    if (key === "Enter" || key === " ") {
      if (activeIndex >= 0) select(activeIndex);
      event.preventDefault();
      return;
    }
    if (key === "Escape" || key === "Tab") {
      closeAndRestore();
      event.preventDefault();
      return;
    }
    if (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      // Typeahead: accumulate printable chars, reset after TYPEAHEAD_RESET_MS.
      const buffer = (typed + key).toLowerCase();
      setTyped(buffer);
      if (typeTimer.current !== null) clearTimeout(typeTimer.current);
      typeTimer.current = setTimeout(() => {
        typeTimer.current = null;
        setTyped("");
      }, TYPEAHEAD_RESET_MS);
      const start = activeIndex >= 0 ? activeIndex : -1;
      for (let step = 1; step <= options.length; step += 1) {
        const index = (start + step) % options.length;
        const option = options[index];
        if (option === undefined || option.disabled === true) continue;
        if (resolveLabel(option.labelKey).toLowerCase().startsWith(buffer)) {
          focusIndex(index);
          break;
        }
      }
      event.preventDefault();
    }
  };

  const selectedLabel = (): string => {
    const option = options.find((candidate) => candidate.value === value);
    return option === undefined ? "" : resolveLabel(option.labelKey);
  };

  return (
    <div
      ref={(node) => {
        rootRef.current = node;
      }}
      id={rootId}
      className={[
        "lumen-dropdown",
        `lumen-dropdown--${density}`,
        invalid && "lumen-dropdown--invalid",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <button
        ref={(element) => {
          triggerRef.current = element;
          if (typeof ref === "function") ref(element);
          else if (ref !== null && ref !== undefined) ref.current = element;
        }}
        type="button"
        className="lumen-dropdown__trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        aria-invalid={invalid || undefined}
        disabled={disabled}
        onClick={() => (open ? closeAndRestore() : openList())}
      >
        <span className="lumen-dropdown__value">{selectedLabel()}</span>
        <span className="lumen-dropdown__arrow" aria-hidden="true" />
      </button>
      {open && (
        <div
          role="listbox"
          aria-label={label}
          className="lumen-dropdown__list"
          onKeyDown={handleListKeyDown}
        >
          {options.map((option, index) => (
            <div
              key={option.value}
              ref={(element) => {
                optionRefs.current[index] = element;
              }}
              role="option"
              tabIndex={activeIndex === index ? 0 : -1}
              aria-selected={option.value === value}
              aria-disabled={option.disabled || undefined}
              className="lumen-dropdown__option"
              onFocus={() => setActiveIndex(index)}
              onClick={() => select(index)}
            >
              {option.icon !== undefined && (
                <span className="lumen-dropdown__option-icon" aria-hidden="true">
                  <Icon name={option.icon} />
                </span>
              )}
              <span className="lumen-dropdown__option-label">{resolveLabel(option.labelKey)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** Alias per card 4.B3 ("Dropdown/Select"). */
export const Select = Dropdown;
