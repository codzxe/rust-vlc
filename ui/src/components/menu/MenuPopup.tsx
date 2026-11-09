import { useEffect, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent, ReactElement } from "react";
import { MenuItem } from "./MenuItem";
import type { MenuEntry, ResolveLabel } from "./types";
import { SUBMENU_CLOSE_DELAY_MS, SUBMENU_OPEN_DELAY_MS } from "./timings";

/**
 * Shared `role="menu"` popup: roving tabindex, ArrowUp/Down/Home/End
 * navigation, ArrowRight/Left + hover for submenus, Escape close with focus
 * restoration, Tab focus trap (card 4.B3), separators skipped, disabled
 * items focusable-but-inert (aria-disabled). Used by MenuBar menus,
 * ContextMenu and nested submenus.
 */
export interface MenuPopupProps<K extends string = string> {
  entries: MenuEntry<K>[];
  resolveLabel: ResolveLabel<K>;
  /** Close this popup; the caller restores focus to its trigger item. */
  onRequestClose: () => void;
  /** Close the whole menu chain (item activation). */
  onDismissAll: () => void;
  /** > 0 = nested submenu: ArrowLeft closes back to the parent item. */
  depth?: number;
  /** Focus the first item on mount (menubar/context menus). @default true */
  autoFocusFirstItem?: boolean;
  className?: string;
  style?: CSSProperties;
  ariaLabel?: string;
  /** Stable prefix for DOM ids (test hooks + aria wiring). */
  idPrefix: string;
}

export function MenuPopup<K extends string>({
  entries,
  resolveLabel,
  onRequestClose,
  onDismissAll,
  depth = 0,
  autoFocusFirstItem = true,
  className = "",
  style,
  ariaLabel,
  idPrefix,
}: MenuPopupProps<K>): ReactElement {
  const [activeIndex, setActiveIndex] = useState(-1);
  const [openSubmenu, setOpenSubmenu] = useState<number | null>(null);
  const itemRefs = useRef<Array<HTMLDivElement | null>>([]);
  const openTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (openTimer.current !== null) clearTimeout(openTimer.current);
      if (closeTimer.current !== null) clearTimeout(closeTimer.current);
    },
    [],
  );

  // Focus first item on mount (WAI-ARIA: opening a menu moves focus in).
  useEffect(() => {
    if (!autoFocusFirstItem) return;
    const first = entries.findIndex((entry) => entry.kind !== "separator");
    if (first >= 0) {
      setActiveIndex(first);
      itemRefs.current[first]?.focus();
    }
    // Mount-time only: menu contents are data-driven and remount per open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const clearHoverTimers = (): void => {
    if (openTimer.current !== null) clearTimeout(openTimer.current);
    if (closeTimer.current !== null) clearTimeout(closeTimer.current);
  };

  const focusIndex = (index: number): void => {
    setActiveIndex(index);
    itemRefs.current[index]?.focus();
  };

  const move = (delta: number): void => {
    if (entries.length === 0) return;
    let index = activeIndex;
    for (let step = 0; step < entries.length; step += 1) {
      index = (index + delta + entries.length) % entries.length;
      if (entries[index]?.kind !== "separator") break;
    }
    focusIndex(index);
  };

  const firstIndex = (): number => entries.findIndex((entry) => entry.kind !== "separator");
  const lastIndex = (): number => {
    for (let index = entries.length - 1; index >= 0; index -= 1) {
      if (entries[index]?.kind !== "separator") return index;
    }
    return -1;
  };

  const activate = (index: number): void => {
    const entry = entries[index];
    if (entry === undefined || entry.kind === "separator" || entry.disabled) return;
    if (entry.submenu !== undefined && entry.submenu.length > 0) {
      clearHoverTimers();
      setOpenSubmenu(index);
      return;
    }
    entry.onSelect?.();
    onDismissAll();
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const { key } = event;
    // Every handled key is terminal: stopPropagation keeps nested popups'
    // keys from also driving the menubar (e.g. ArrowRight opening a submenu
    // must not switch the open top-level menu).
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
      const first = firstIndex();
      if (first >= 0) focusIndex(first);
      handled();
      return;
    }
    if (key === "End") {
      const last = lastIndex();
      if (last >= 0) focusIndex(last);
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
      onRequestClose();
      handled();
      return;
    }
    if (key === "ArrowLeft") {
      if (depth > 0) {
        onRequestClose();
        handled();
      }
      return;
    }
    if (key === "ArrowRight") {
      const entry = entries[activeIndex];
      if (entry !== undefined && entry.kind !== "separator" && (entry.submenu?.length ?? 0) > 0) {
        clearHoverTimers();
        setOpenSubmenu(activeIndex);
        handled();
      }
      return;
    }
    if (key === "Enter" || key === " ") {
      if (activeIndex >= 0) activate(activeIndex);
      handled();
    }
  };

  const hoverEnter = (index: number): void => {
    const entry = entries[index];
    if (entry === undefined || entry.kind === "separator" || (entry.submenu?.length ?? 0) === 0) return;
    if (closeTimer.current !== null) {
      clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
    if (openSubmenu === index) return;
    if (openTimer.current !== null) clearTimeout(openTimer.current);
    openTimer.current = setTimeout(() => {
      openTimer.current = null;
      setOpenSubmenu(index);
    }, SUBMENU_OPEN_DELAY_MS);
  };

  const hoverLeave = (index: number): void => {
    if (openTimer.current !== null) {
      clearTimeout(openTimer.current);
      openTimer.current = null;
    }
    if (openSubmenu !== index) return;
    if (closeTimer.current !== null) clearTimeout(closeTimer.current);
    closeTimer.current = setTimeout(() => {
      closeTimer.current = null;
      setOpenSubmenu(null);
    }, SUBMENU_CLOSE_DELAY_MS);
  };

  return (
    <div
      role="menu"
      aria-label={ariaLabel}
      className={["lumen-menu", className].filter(Boolean).join(" ")}
      style={style}
      onKeyDown={handleKeyDown}
    >
      {entries.map((entry, index) =>
        entry.kind === "separator" ? (
          <div key={entry.id} role="separator" className="lumen-menu__separator" />
        ) : (
          <div
            key={entry.id}
            className="lumen-menu__host"
            onFocus={() => setActiveIndex(index)}
            onMouseEnter={() => hoverEnter(index)}
            onMouseLeave={() => hoverLeave(index)}
          >
            <MenuItem
              id={`${idPrefix}-item-${index}`}
              entry={entry}
              resolveLabel={resolveLabel}
              active={activeIndex === index}
              hasSubmenu={(entry.submenu?.length ?? 0) > 0}
              submenuOpen={openSubmenu === index}
              itemRef={(element) => {
                itemRefs.current[index] = element;
              }}
              onClick={() => activate(index)}
            />
            {openSubmenu === index && entry.submenu !== undefined && (
              <MenuPopup
                entries={entry.submenu}
                resolveLabel={resolveLabel}
                idPrefix={`${idPrefix}-sub-${index}`}
                depth={depth + 1}
                className="lumen-menu--submenu"
                ariaLabel={resolveLabel(entry.labelKey)}
                onRequestClose={() => {
                  setOpenSubmenu(null);
                  focusIndex(index);
                }}
                onDismissAll={onDismissAll}
              />
            )}
          </div>
        ),
      )}
    </div>
  );
}
