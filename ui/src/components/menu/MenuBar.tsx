import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement, Ref } from "react";
import { MenuPopup } from "./MenuPopup";
import { identityResolveLabel } from "./types";
import type { MenuDensity, MenuEntry, ResolveLabel } from "./types";
import "./menu.css";

/** One top-level menubar menu: trigger + entries (doc 05 §8.3 menu tree). */
export interface MenuBarMenu<K extends string = string> {
  id: string;
  labelKey: K;
  entries: MenuEntry<K>[];
}

export interface MenuBarProps<K extends string = string> {
  menus: MenuBarMenu<K>[];
  /**
   * i18n seam: resolves `labelKey`s (wire `useMsg()` here, doc 05 §8.3).
   * @default identity (keys render verbatim — dev mode)
   */
  resolveLabel?: ResolveLabel<K>;
  /** Row scale for the open popups (W26). @default "comfortable" */
  density?: MenuDensity;
  id?: string;
  className?: string;
  /** React 19 ref-as-prop on the menubar container. */
  ref?: Ref<HTMLDivElement>;
}

/**
 * MenuBar (doc 08 §267 overlays: "MenuBar (app menu)") — horizontal
 * `role="menubar"`, WAI-ARIA menubar keyboard model: ArrowLeft/Right move
 * between top items (switching the open menu when one is open), ArrowDown /
 * Enter / Space open, Escape closes and restores focus to the trigger.
 * Open menus are role="menu" popups with roving tabindex, submenus and a Tab
 * focus trap (MenuPopup). Portal-free absolute positioning (doc 08 specifies
 * no portals).
 */
export function MenuBar<K extends string>({
  menus,
  resolveLabel = identityResolveLabel,
  density = "comfortable",
  id,
  className = "",
  ref,
}: MenuBarProps<K>): ReactElement {
  const autoId = useId();
  const rootId = id ?? autoId;
  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRefs = useRef<Array<HTMLDivElement | null>>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  const focusTrigger = (index: number): void => {
    triggerRefs.current[index]?.focus();
  };

  const closeMenu = (index: number): void => {
    setOpenIndex(null);
    focusTrigger(index);
  };

  // Outside pointer dismiss (focus is not stolen — the click target keeps it).
  useEffect(() => {
    if (openIndex === null) return;
    const onPointerDown = (event: PointerEvent): void => {
      if (rootRef.current !== null && !rootRef.current.contains(event.target as Node)) {
        setOpenIndex(null);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [openIndex]);

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const { key } = event;
    if (key === "ArrowRight" || key === "ArrowLeft") {
      const delta = key === "ArrowRight" ? 1 : -1;
      const next = (activeIndex + delta + menus.length) % menus.length;
      setActiveIndex(next);
      triggerRefs.current[next]?.focus();
      if (openIndex !== null) setOpenIndex(next);
      event.preventDefault();
      return;
    }
    if (key === "ArrowDown" || key === "Enter" || key === " ") {
      setOpenIndex(activeIndex);
      event.preventDefault();
      return;
    }
    if (key === "Escape" && openIndex !== null) {
      const target = openIndex;
      setOpenIndex(null);
      focusTrigger(target);
      event.preventDefault();
    }
  };

  return (
    <div
      ref={(node) => {
        rootRef.current = node;
        if (typeof ref === "function") ref(node);
        else if (ref !== null && ref !== undefined) ref.current = node;
      }}
      id={rootId}
      role="menubar"
      className={["lumen-menubar", className].filter(Boolean).join(" ")}
      onKeyDown={handleKeyDown}
    >
      {menus.map((menu, index) => (
        <div key={menu.id} className="lumen-menubar__item-wrapper">
          <div
            ref={(element) => {
              triggerRefs.current[index] = element;
            }}
            role="menuitem"
            tabIndex={activeIndex === index ? 0 : -1}
            aria-haspopup="true"
            aria-expanded={openIndex === index}
            className="lumen-menubar__trigger"
            onFocus={() => setActiveIndex(index)}
            onClick={() => {
              setActiveIndex(index);
              setOpenIndex(openIndex === index ? null : index);
            }}
          >
            {resolveLabel(menu.labelKey)}
          </div>
          {openIndex === index && (
            <MenuPopup
              entries={menu.entries}
              resolveLabel={resolveLabel}
              idPrefix={`${rootId}-${menu.id}`}
              density={density}
              className="lumen-menubar__popup"
              ariaLabel={resolveLabel(menu.labelKey)}
              onRequestClose={() => closeMenu(index)}
              onDismissAll={() => closeMenu(index)}
            />
          )}
        </div>
      ))}
    </div>
  );
}
