import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement, Ref } from "react";
import { Icon } from "../icon/Icon";
import type { IconName } from "../icon/icons";
import { identityResolveLabel } from "./types";
import type { MenuDensity, ResolveLabel } from "./types";
import { TYPEAHEAD_RESET_MS } from "./timings";
import "./menu.css";

/** Scrollport cap for long option lists (w26.g17): ~6-7 rows visible, the
 * rest scroll — a popup must never run past the viewport edge. */
const DROPDOWN_LIST_MAX_HEIGHT = 320;

/**
 * w26.i16 (h15's HDR NO): the open-direction flip must measure the CLIPPING
 * CONTAINER, not the viewport — `.lumen-dialog { overflow: hidden }` +
 * `.player-dialog__body { overflow-y: auto }` clip lists that still "fit the
 * window" (HDR at rest: list bottom 888 vs dialog bottom 646 — 5/6 options
 * painted nowhere, `elementFromPoint` hit the scrim). Walk up from the
 * trigger to the nearest ancestor that actually clips (overflow /
 * overflow-x / overflow-y ≠ visible) and measure against its rect; the
 * viewport is the fallback (jsdom: nothing clips → viewport, tests unchanged).
 */
const clippingBounds = (from: HTMLElement): { top: number; bottom: number } => {
  if (typeof window === "undefined") return { top: 0, bottom: 0 };
  let node = from.parentElement;
  while (node !== null && node !== document.documentElement) {
    const style = window.getComputedStyle(node);
    const clips =
      (style.overflow !== "" && style.overflow !== "visible") ||
      (style.overflowX !== "" && style.overflowX !== "visible") ||
      (style.overflowY !== "" && style.overflowY !== "visible");
    if (clips) {
      const rect = node.getBoundingClientRect();
      return { top: rect.top, bottom: rect.bottom };
    }
    node = node.parentElement;
  }
  return { top: 0, bottom: window.innerHeight };
};

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
  /**
   * w26.i16 — preferred open side. `Setting Preset.svg` pins its picker UP
   * (chevron-up open state); most hosts ride the measured flip (default
   * "down"). The preference applies only while the preferred side FITS the
   * clipping container — the measurement-driven flip still wins whenever
   * it does not (a pinned preference may never clip the list).
   * @default "down"
   */
  preferredDirection?: "down" | "up";
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
  preferredDirection = "down",
  id,
  className = "",
  ref,
}: DropdownProps<K>): ReactElement {
  const autoId = useId();
  const rootId = id ?? autoId;
  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const optionRefs = useRef<Array<HTMLDivElement | null>>([]);
  const typeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [typed, setTyped] = useState("");
  /* w26.g17/g19 P1 Lead fix: flip-up when the viewport has more room above
   * the trigger than below, so lists never clip under the viewport edge
   * (HDR dropdown inside the tools dialog, sorting menu at 1024×768). */
  const [direction, setDirection] = useState<"down" | "up">("down");

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
    const node = optionRefs.current[index];
    node?.focus();
    // w26.g17: keep the active row visible in the scrollport (jsdom does not
    // implement scrollIntoView — guard so vitest stays green).
    node?.scrollIntoView?.({ block: "nearest" });
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

  // w26.g17: open the scrollport at the top. w26.i16 root fix (h13/i06's
  // "Full treble" repro): this used to be an INLINE ref callback
  // (`ref={(node) => { node.scrollTop = 0 }}`) — a fresh closure every render
  // re-fires on every re-render, yanking the scrollport to the top mid-click
  // so scroll-required rows escaped the cursor (equalizer.css sized its list
  // to 372px as a workaround — revert that to the frame's 348px now). An
  // effect keyed on `open` runs ONCE per open; the stable ref never re-fires.
  useEffect(() => {
    if (!open) return;
    if (listRef.current !== null) listRef.current.scrollTop = 0;
  }, [open]);

  // Open-direction measurement (w26.g17/g19): pick the side with room —
  // against the nearest CLIPPING CONTAINER (w26.i16, h15's HDR repro), not
  // the viewport (a dialog's overflow:hidden body is what actually clips).
  useLayoutEffect(() => {
    if (!open) return;
    const trigger = triggerRef.current;
    if (trigger === null || typeof window === "undefined") return;
    const rect = trigger.getBoundingClientRect();
    const bounds = clippingBounds(trigger);
    const rowPitch = density === "compact" ? 42 : 46; /* 44+2 / 40+2 — Dropdown.svg grammar */
    const estimated = Math.min(options.length * rowPitch + 8, DROPDOWN_LIST_MAX_HEIGHT);
    const below = bounds.bottom - rect.bottom;
    const above = rect.top - bounds.top;
    // w26.i16: the host's preferred side wins while it fits (Setting
    // Preset.svg pins its picker up); otherwise the measured roomier side.
    const measured: "down" | "up" = below < estimated && above > below ? "up" : "down";
    const preferredFits = preferredDirection === "up" ? above >= estimated : below >= estimated;
    setDirection(preferredFits ? preferredDirection : measured);
    // Deps: measure per open — density changes re-estimate (no trigger-move
    // listener; a moved trigger re-measures on the next open).
  }, [open, options.length, density, preferredDirection]);

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
      /* w26.g19 P1-2 Lead fix: preventDefault alone does not stop the event
       * from BUBBLING — host dialogs (subtitle picker, settings window) also
       * listen for Escape and closed the whole dialog while the dropdown was
       * still open. stopPropagation keeps the close scoped to the dropdown. */
      event.stopPropagation();
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
          ref={listRef}
          role="listbox"
          aria-label={label}
          data-direction={direction}
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
