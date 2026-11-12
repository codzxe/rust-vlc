// w26.c3 — VisualizerModeSelect: the "Audio Visualizer.svg" mode dropdown
// (w26.a20 §A7 — the a19 A9 Dropdown grammar the frame reuses verbatim),
// with the frame's five options and their two-tone 20px leading icons.
//
// GEOMETRY (frame, "Audio Visualizer.svg" / "EQ presets.svg" family):
//   trigger  362×48 rx6 #090909 + inner 358×44 rx4 #191919 (2px ring) ·
//            label #8F8F8F ≈15px @ 15px inset · chevron-∨ 10×5 sw1.5
//            #8F8F8F @ 21px right inset (field coords);
//   list     rx6 #191919 + shadow 0/+4 σ6.9 · 2px inset · rows 358×44 rx4
//            @ 46 pitch (2px gaps) · selected #1F1F1F + check 14×11 sw2
//            #D37918 @ 19px right inset · icons 20px @ 16px · labels
//            #8F8F8F ≈15px @ ~13px past the icon.
//
// Lane-scoped (not the shared Dropdown): the frame's two-tone mode previews
// are custom svg nodes the primitive's `icon?: IconName` API cannot carry
// (the additive ask the w26.c1 log left with the menu lane). The
// interaction model MIRRORS the primitive: combobox trigger +
// aria-haspopup="listbox", popup role="listbox" with roving tabindex,
// arrows/Home/End, Enter/Space select, Escape/Tab close with focus
// restoration, outside-pointer dismiss. (Typeahead is the one omitted
// behavior — five options; noted in the lane log.)

import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement } from "react";
import { Icon } from "../../../../components/icon/Icon";
import { VISUALIZER_MODES } from "./types";
import type { VisualizerLabelKey, VisualizerMode } from "./types";

/** Public props (mountable; state lives in the parent tool). */
export interface VisualizerModeSelectProps {
  /** Selected mode id. */
  value: VisualizerMode;
  /** Commit seam — fires with the picked mode. */
  onChange: (mode: VisualizerMode) => void;
  /** Accessible name for the trigger + listbox (already resolved). */
  label: string;
  /** Resolves option label keys (the useVisualizerMsg seam). */
  resolveLabel: (key: VisualizerLabelKey) => string;
  /** Stable id for the root; generated when omitted. */
  id?: string;
  /** Extra classes on the root. */
  className?: string;
}

export function VisualizerModeSelect({
  value,
  onChange,
  label,
  resolveLabel,
  id: idProp,
  className = "",
}: VisualizerModeSelectProps): ReactElement {
  const autoId = useId();
  const rootId = idProp ?? autoId;
  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const optionRefs = useRef<Array<HTMLDivElement | null>>([]);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);

  // Outside pointer dismiss (the shared Dropdown's model).
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

  const focusIndex = (index: number): void => {
    setActiveIndex(index);
    optionRefs.current[index]?.focus();
  };

  const closeAndRestore = (): void => {
    setOpen(false);
    triggerRef.current?.focus();
  };

  const select = (index: number): void => {
    const mode = VISUALIZER_MODES[index];
    if (mode === undefined) return;
    onChange(mode.id);
    closeAndRestore();
  };

  const openList = (): void => {
    const selectedIndex = VISUALIZER_MODES.findIndex((mode) => mode.id === value);
    setOpen(true);
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : 0);
  };

  // Focus the selected option when the list opens (WAI-ARIA listbox) —
  // and reveal the WHOLE list first: inside the scrolling tools-dialog body
  // the absolutely-positioned list can open with its tail below the fold
  // (w26.i07, h14 defect 4: measured 164px under the body's clip — 3.5 of 5
  // options invisible to pointer users). `block: "nearest"` scrolls the
  // minimum needed and is a no-op when the list already fits; focusing the
  // option afterwards cannot re-scroll (it is on screen by then). jsdom has
  // no scrollIntoView — guarded so tests keep running.
  useEffect(() => {
    if (!open) return;
    const list = listRef.current;
    if (list !== null && typeof list.scrollIntoView === "function") {
      list.scrollIntoView({ block: "nearest" });
    }
    if (activeIndex >= 0) optionRefs.current[activeIndex]?.focus();
    // Open-transition only — later activeIndex changes focus themselves.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const handleListKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const { key } = event;
    const count = VISUALIZER_MODES.length;
    if (key === "ArrowDown" || key === "ArrowUp") {
      const base = activeIndex >= 0 ? activeIndex : key === "ArrowDown" ? -1 : count;
      const delta = key === "ArrowDown" ? 1 : -1;
      const next = (base + delta + count) % count;
      focusIndex(next);
      event.preventDefault();
      return;
    }
    if (key === "Home") {
      focusIndex(0);
      event.preventDefault();
      return;
    }
    if (key === "End") {
      focusIndex(count - 1);
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
    }
  };

  const selected = VISUALIZER_MODES.find((mode) => mode.id === value);
  const SelectedIcon = selected?.icon;

  return (
    <div
      ref={(node) => {
        rootRef.current = node;
      }}
      id={rootId}
      className={["viz-select", open ? "viz-select--open" : "", className]
        .filter(Boolean)
        .join(" ")}
    >
      {/* frame state=default: 362×48 rx6 #090909 trigger + 358×44 rx4
       * #191919 field (2px ring). The field carries the content. */}
      <button
        ref={(element) => {
          triggerRef.current = element;
        }}
        type="button"
        className="viz-select__trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        onClick={() => (open ? closeAndRestore() : openList())}
      >
        <span className="viz-select__field">
          {SelectedIcon !== undefined && (
            <SelectedIcon className="viz-select__selected-icon" />
          )}
          <span className="viz-select__value">
            {selected === undefined ? "" : resolveLabel(selected.labelKey)}
          </span>
          <Icon name="chevron-down" size={24} className="viz-select__chevron" />
        </span>
      </button>
      {open && (
        <div
          ref={(node) => {
            listRef.current = node;
          }}
          role="listbox"
          aria-label={label}
          className="viz-select__list"
          onKeyDown={handleListKeyDown}
        >
          {VISUALIZER_MODES.map((mode, index) => {
            const ModeIcon = mode.icon;
            const selectedRow = mode.id === value;
            return (
              <div
                key={mode.id}
                ref={(element) => {
                  optionRefs.current[index] = element;
                }}
                role="option"
                aria-selected={selectedRow}
                tabIndex={activeIndex === index ? 0 : -1}
                className="viz-select__option"
                data-mode={mode.id}
                onFocus={() => setActiveIndex(index)}
                onClick={() => select(index)}
              >
                {ModeIcon !== undefined && <ModeIcon className="viz-select__icon" />}
                <span className="viz-select__option-label">
                  {resolveLabel(mode.labelKey)}
                </span>
                {selectedRow && (
                  /* frame: Vector 44 — 14×11 check, sw2 #D37918 */
                  <Icon
                    name="check"
                    size={24}
                    variant="strong"
                    className="viz-select__check"
                  />
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
