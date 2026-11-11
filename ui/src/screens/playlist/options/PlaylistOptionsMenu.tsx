// w26.b10 — anchored single-select options menu (`Sorting Options.svg` /
// `View Options.svg`): a compact `role="menu"` popup under an icon-button
// trigger, built in the shared Figma menu grammar (menu.css classes:
// r10–12 panel, r6 rows, hairline border, overlay shadow) at the option
// frames' 265px / 48px compact scale (menu lane states.md §"Known gaps"
// sanctions app-layer 48px overrides).
//
// Why not the shared MenuPopup: both frames pin rows that carry a LEADING
// ICON *and* a right-aligned orange check on the selected row
// (View Options.svg row 2 = list icon + check; MenuItem renders either an
// icon or a check, never both — logged as a fix-needed for the menu lane).
// The keyboard model below mirrors MenuPopup's APG implementation exactly:
// ArrowUp/Down cycle (disabled skipped), Home/End, Enter/Space activate,
// Escape closes and restores focus to the trigger, Tab wraps inside the
// menu (focus trap), roving tabindex.

import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement } from "react";
import { IconButton } from "../../../components/button";
import { Icon } from "../../../components/icon";
import type { IconName } from "../../../components/icon/icons";
import "../../../components/menu/menu.css";
import "./options.css";

/** One selectable row. `id` is the value (`onSelect(id)`). */
export interface OptionsMenuEntry<K extends string = string> {
  id: string;
  /** i18n key resolved through `resolveLabel` (doc 05 §8.3: keys only). */
  labelKey: K;
  /** Leading line icon (View Options.svg rows carry grid/list icons). */
  icon?: IconName;
  disabled?: boolean;
}

export interface PlaylistOptionsMenuProps<K extends string = string> {
  entries: readonly OptionsMenuEntry<K>[];
  /** Selected entry id — its row renders the orange check + selected fill. */
  value: string;
  /** Fires on click / Enter / Space; the menu dismisses afterwards. */
  onSelect: (id: string) => void;
  /** i18n seam — wire the lane resolver (`useOptionsMsg`). */
  resolveLabel: (key: K) => string;
  /** Accessible name for the popup itself. */
  menuLabel: string;
  /** Accessible name for the trigger button (icon-only control). */
  triggerLabel: string;
  /** Trigger glyph — e.g. the current view mode's icon. */
  triggerIcon: IconName;
  /** Popup alignment against the trigger. @default "start" */
  align?: "start" | "end";
  /** Popup placement. @default "below" (frames sit under the meta row). */
  placement?: "below" | "above";
  /** Extra classes for the wrapper (density/layout hooks). */
  className?: string;
  id?: string;
}

export function PlaylistOptionsMenu<K extends string>({
  entries,
  value,
  onSelect,
  resolveLabel,
  menuLabel,
  triggerLabel,
  triggerIcon,
  align = "start",
  placement = "below",
  className = "",
  id,
}: PlaylistOptionsMenuProps<K>): ReactElement {
  const autoId = useId();
  const rootId = id ?? autoId;
  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const itemRefs = useRef<Array<HTMLDivElement | null>>([]);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);

  // Outside pointer dismiss (focus is not stolen).
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

  const enabled = (index: number): boolean => entries[index]?.disabled !== true;

  const focusIndex = (index: number): void => {
    setActiveIndex(index);
    itemRefs.current[index]?.focus();
  };

  const move = (delta: number): void => {
    if (entries.length === 0) return;
    let index = activeIndex;
    for (let step = 0; step < entries.length; step += 1) {
      index = (index + delta + entries.length) % entries.length;
      if (enabled(index)) break;
    }
    focusIndex(index);
  };

  const openMenu = (): void => {
    // Focus the selected row when it is enabled (Dropdown precedent);
    // otherwise the first enabled row.
    const selected = entries.findIndex((entry) => entry.id === value);
    const initial = selected >= 0 && enabled(selected) ? selected : entries.findIndex((entry) => entry.disabled !== true);
    setOpen(true);
    setActiveIndex(initial);
  };

  const closeAndRestore = (): void => {
    setOpen(false);
    triggerRef.current?.focus();
  };

  const activate = (index: number): void => {
    const entry = entries[index];
    if (entry === undefined || entry.disabled) return;
    onSelect(entry.id);
    closeAndRestore();
  };

  // Focus the initial row once the popup mounts (WAI-ARIA: opening a menu
  // moves focus in).
  useEffect(() => {
    if (!open) return;
    if (activeIndex >= 0) itemRefs.current[activeIndex]?.focus();
    // Open-transition only — later activeIndex changes focus themselves.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const { key } = event;
    const handled = (): void => {
      event.preventDefault();
      event.stopPropagation();
    };
    if (key === "ArrowDown" || key === "ArrowUp") {
      move(key === "ArrowDown" ? 1 : -1);
      handled();
      return;
    }
    if (key === "Home") {
      const first = entries.findIndex((entry) => entry.disabled !== true);
      if (first >= 0) focusIndex(first);
      handled();
      return;
    }
    if (key === "End") {
      for (let index = entries.length - 1; index >= 0; index -= 1) {
        if (enabled(index)) {
          focusIndex(index);
          break;
        }
      }
      handled();
      return;
    }
    if (key === "Tab") {
      // Focus trap (card 4.B3): Tab cycles inside the open menu.
      move(event.shiftKey ? -1 : 1);
      handled();
      return;
    }
    if (key === "Escape") {
      closeAndRestore();
      handled();
      return;
    }
    if (key === "Enter" || key === " ") {
      if (activeIndex >= 0) activate(activeIndex);
      handled();
    }
  };

  return (
    <div
      ref={(node) => {
        rootRef.current = node;
      }}
      id={rootId}
      className={["lumen-plo", className].filter(Boolean).join(" ")}
    >
      <IconButton
        ref={(element) => {
          triggerRef.current = element;
        }}
        label={triggerLabel}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => (open ? closeAndRestore() : openMenu())}
      >
        <Icon name={triggerIcon} size={20} />
      </IconButton>
      {open && (
        <div
          role="menu"
          aria-label={menuLabel}
          className={[
            "lumen-menu",
            "lumen-plo__menu",
            `lumen-plo__menu--${align}`,
            `lumen-plo__menu--${placement}`,
          ].join(" ")}
          onKeyDown={handleKeyDown}
        >
          {entries.map((entry, index) => {
            const selected = entry.id === value;
            return (
              <div
                key={entry.id}
                ref={(element) => {
                  itemRefs.current[index] = element;
                }}
                role="menuitemradio"
                tabIndex={activeIndex === index ? 0 : -1}
                aria-checked={selected}
                aria-disabled={entry.disabled || undefined}
                className={["lumen-menu__item", "lumen-plo__item", selected ? "lumen-plo__item--selected" : ""]
                  .filter(Boolean)
                  .join(" ")}
                onFocus={() => setActiveIndex(index)}
                onClick={() => activate(index)}
              >
                {entry.icon !== undefined && (
                  <span className="lumen-menu__icon lumen-plo__icon" aria-hidden="true">
                    <Icon name={entry.icon} />
                  </span>
                )}
                <span className="lumen-menu__label">{resolveLabel(entry.labelKey)}</span>
                {selected && <span className="lumen-plo__check" aria-hidden="true" />}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
