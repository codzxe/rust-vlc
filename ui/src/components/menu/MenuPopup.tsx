import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent, ReactElement } from "react";
import { MenuItem } from "./MenuItem";
import type { MenuDensity, MenuEntry, ResolveLabel } from "./types";
import { SUBMENU_CLOSE_DELAY_MS, SUBMENU_OPEN_DELAY_MS } from "./timings";

/**
 * Shared `role="menu"` popup: roving tabindex, ArrowUp/Down/Home/End
 * navigation, ArrowRight/Left + hover for submenus, Escape close with focus
 * restoration, Tab focus trap (card 4.B3), separators skipped, disabled
 * items focusable-but-inert (aria-disabled). Used by MenuBar menus,
 * ContextMenu and nested submenus.
 *
 * Viewport clamping (w26.f07): every popup measures itself on mount (a
 * submenu opening IS a MenuPopup mount) and repositions before paint, so
 * nothing ever opens clipped off-window (e02 F1 / e12 F1+F2):
 *  * pointer-anchored popups — `style.left/top` in px are pointer coordinates
 *    for a fixed-position popup (the ContextMenu / player canvas-menu
 *    contract); they FLIP across the pointer (up / left) when the panel would
 *    overflow the bottom/right, shifting to the viewport margin as a last
 *    resort;
 *  * submenus (depth > 0) — the CSS anchor opens right of the parent row;
 *    they flip fully to the LEFT of that row near the right edge and shift UP
 *    near the bottom.
 */

/** Viewport margin a clamped popup keeps from every window edge (w26.f07) —
 * structural, no token. */
const VIEWPORT_MARGIN_PX = 8;

/** Gap between a submenu and its parent row when flipped to the LEFT side —
 * mirrors the CSS `margin-left: var(--space-grid)` right-side gap. */
const SUBMENU_FLIP_GAP_PX = 4;

/** `style.left`/`style.top` as a px number, else null (percent/calc values
 * are anchors, not pointer coordinates — leave them to the CSS). */
const pointerCoord = (value: string | number | undefined): number | null => {
  if (typeof value === "number") return value;
  if (typeof value === "string" && value.endsWith("px")) {
    const parsed = Number.parseFloat(value);
    return Number.isNaN(parsed) ? null : parsed;
  }
  return null;
};

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
  /**
   * w26.k02 (j02 J02-9, APG menubar): focus the LAST non-separator item on
   * mount — the menubar's ArrowUp open path. Takes precedence over
   * `autoFocusFirstItem` (mount-time only; both default to first-focus
   * semantics for every existing host). Additive.
   * @default false
   */
  autoFocusLastItem?: boolean;
  /** Row scale (W26 figma grammar): comfortable = big player menus,
  *  compact = ~40px rows. @default "comfortable" */
  density?: MenuDensity;
  className?: string;
  ariaLabel?: string;
  /** Stable prefix for DOM ids (test hooks + aria wiring). */
  idPrefix: string;
  /**
   * Popup position. `left`/`top` in px are read as POINTER coordinates of a
   * fixed-position popup (the ContextMenu / canvas-menu contract) and get
   * viewport-clamped on mount (flip up/left, shift to the margin). Omit them
   * for CSS-anchored (menubar-style) popups.
   */
  style?: CSSProperties;
}

export function MenuPopup<K extends string>({
  entries,
  resolveLabel,
  onRequestClose,
  onDismissAll,
  depth = 0,
  autoFocusFirstItem = true,
  autoFocusLastItem = false,
  density = "comfortable",
  className = "",
  style,
  ariaLabel,
  idPrefix,
}: MenuPopupProps<K>): ReactElement {
  const [activeIndex, setActiveIndex] = useState(-1);
  const [openSubmenu, setOpenSubmenu] = useState<number | null>(null);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const itemRefs = useRef<Array<HTMLDivElement | null>>([]);
  const openTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // w26.f07 — viewport clamping: measure via getBoundingClientRect on mount
  // and reposition BEFORE paint (useLayoutEffect). Re-runs when the caller
  // supplies a fresh `style` object; idempotent — the flip origin is the
  // `style` prop, never the mutated DOM style.
  useLayoutEffect(() => {
    const el = rootRef.current;
    if (el === null) return;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const margin = VIEWPORT_MARGIN_PX;
    const pointerX = pointerCoord(style?.left);
    const pointerY = pointerCoord(style?.top);

    if (depth === 0 && pointerX !== null && pointerY !== null) {
      // Pointer-anchored popup: FLIP across the pointer when the panel would
      // overflow the bottom/right; clamp at the margin as a last resort.
      const rect = el.getBoundingClientRect();
      const left =
        pointerX + rect.width > vw - margin ? Math.max(margin, pointerX - rect.width) : Math.max(margin, pointerX);
      const top =
        pointerY + rect.height > vh - margin ? Math.max(margin, pointerY - rect.height) : Math.max(margin, pointerY);
      el.style.left = `${left}px`;
      el.style.top = `${top}px`;
      return;
    }

    if (depth > 0) {
      // Submenu: the CSS anchor opens right of the parent row; flip fully
      // LEFT of that row near the right edge (a transform keeps the CSS
      // anchor for the non-overflow case and for hosts that already flip via
      // their own CSS) and shift UP near the bottom. Both keep at least the
      // margin on the opposite edge.
      const rect = el.getBoundingClientRect();
      let dx = 0;
      let dy = 0;
      if (rect.right > vw - margin) {
        const hostLeft = el.parentElement?.getBoundingClientRect().left ?? rect.left;
        const targetLeft = hostLeft - SUBMENU_FLIP_GAP_PX - rect.width;
        dx = Math.max(targetLeft, margin) - rect.left;
      }
      if (rect.bottom > vh - margin) {
        dy = Math.max(margin - rect.top, vh - margin - rect.height - rect.top);
      }
      if (dx !== 0 || dy !== 0) {
        el.style.transform = `translate(${dx}px, ${dy}px)`;
      }
    }
  }, [depth, style]);

  useEffect(
    () => () => {
      if (openTimer.current !== null) clearTimeout(openTimer.current);
      if (closeTimer.current !== null) clearTimeout(closeTimer.current);
    },
    [],
  );

  // Focus the opening item on mount (WAI-ARIA: opening a menu moves focus
  // in). w26.k02 (J02-9): ArrowUp-opened menubar menus land on the LAST
  // non-separator item (APG menubar); every other path keeps first-item
  // focus.
  useEffect(() => {
    if (autoFocusLastItem) {
      const last = lastIndex();
      if (last >= 0) {
        setActiveIndex(last);
        itemRefs.current[last]?.focus();
      }
      return;
    }
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
      ref={rootRef}
      role="menu"
      aria-label={ariaLabel}
      className={["lumen-menu", `lumen-menu--${density}`, className].filter(Boolean).join(" ")}
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
                density={density}
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
