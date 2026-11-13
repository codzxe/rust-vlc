import { useEffect, useId, useRef, useState } from "react";
import type { ReactElement, Ref } from "react";
import { MenuPopup } from "./MenuPopup";
import { identityResolveLabel } from "./types";
import type { MenuDensity, MenuEntry, ResolveLabel } from "./types";
import "./menu.css";

export interface ContextMenuProps<K extends string = string> {
  entries: MenuEntry<K>[];
  /**
   * i18n seam: resolves entry `labelKey`s (wire `useMsg()`, doc 05 §8.3).
   * @default identity
   */
  resolveLabel?: ResolveLabel<K>;
  /** Accessible name for the popup (already-resolved string). */
  ariaLabel?: string;
  /** Row scale (W26): comfortable = big player context menus. @default "comfortable" */
  density?: MenuDensity;
  children: ReactElement | ReactElement[];
  className?: string;
  /** React 19 ref-as-prop on the wrapper. */
  ref?: Ref<HTMLDivElement>;
}

/**
 * ContextMenu (doc 08 §115: 265×296 rx10 `#191919` @ 0.95, items 48 high,
 * hover `#1F1F1F`; figma exports "context menu" / "file|folder|playlist
 * context menu" / "right-click menu") — right-click opens a `role="menu"`
 * popup at the pointer coordinates (fixed positioning, portal-free). Full
 * MenuPopup keyboard model inside; Escape closes and restores focus to the
 * previously-focused element; outside pointer dismisses.
 */
export function ContextMenu<K extends string>({
  entries,
  resolveLabel = identityResolveLabel,
  ariaLabel,
  density = "comfortable",
  children,
  className = "",
  ref,
}: ContextMenuProps<K>): ReactElement {
  const autoId = useId();
  const rootRef = useRef<HTMLDivElement | null>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  /* w26.k02 (j02 J02-10): per-instance id prefix (the MenuBar/Dropdown
   * useId pattern) — the old static "lumen-context-menu" prefix produced
   * duplicate item DOM ids across co-existing instances. */
  const idPrefix = `lumen-context-menu-${autoId}`;

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

  const closeAndRestore = (): void => {
    setOpen(false);
    previousFocus.current?.focus();
  };

  return (
    <div
      ref={(node) => {
        rootRef.current = node;
        if (typeof ref === "function") ref(node);
        else if (ref !== null && ref !== undefined) ref.current = node;
      }}
      className={["lumen-context-menu", className].filter(Boolean).join(" ")}
      onContextMenu={(event) => {
        event.preventDefault();
        previousFocus.current = document.activeElement as HTMLElement | null;
        setPosition({ x: event.clientX, y: event.clientY });
        setOpen(true);
      }}
    >
      {children}
      {open && (
        <MenuPopup
          entries={entries}
          resolveLabel={resolveLabel}
          idPrefix={idPrefix}
          density={density}
          className="lumen-context-menu__popup"
          style={{ left: position.x, top: position.y }}
          ariaLabel={ariaLabel}
          onRequestClose={closeAndRestore}
          onDismissAll={closeAndRestore}
        />
      )}
    </div>
  );
}
