import type { ReactElement } from "react";
import type { MenuEntryBase, ResolveLabel } from "./types";

/**
 * Shared menu-item renderer (doc 08 §259 lists MenuItem as a primitive).
 * Presentational + click activation; keyboard navigation (roving tabindex,
 * arrows, Home/End, Escape, focus trap) is owned by the containing popup
 * (MenuPopup) so every menu surface shares one behavior implementation.
 *
 * Roles per WAI-ARIA / doc 08 §4.5:
 * - plain item → `menuitem`
 * - checkbox / `checked` present → `menuitemcheckbox` + `aria-checked`
 * - radio → `menuitemradio` + `aria-checked`
 * - submenu parents get `aria-haspopup="true"` + `aria-expanded`
 */
export interface MenuItemProps<K extends string = string> {
  entry: MenuEntryBase<K>;
  resolveLabel: ResolveLabel<K>;
  /** Roving tabindex (doc 08 §4.5): only the active item is tab-reachable. */
  active: boolean;
  id?: string;
  hasSubmenu?: boolean;
  submenuOpen?: boolean;
  /** Callback ref to the focusable item element (roving-focus wiring). */
  itemRef?: (element: HTMLDivElement | null) => void;
  onClick?: () => void;
}

export function MenuItem<K extends string>({
  entry,
  resolveLabel,
  active,
  id,
  hasSubmenu = false,
  submenuOpen = false,
  itemRef,
  onClick,
}: MenuItemProps<K>): ReactElement {
  const checkable = entry.kind === "checkbox" || entry.kind === "radio" || entry.checked !== undefined;
  const role = entry.kind === "radio" ? "menuitemradio" : checkable ? "menuitemcheckbox" : "menuitem";
  return (
    <div
      id={id}
      ref={itemRef}
      role={role}
      tabIndex={active ? 0 : -1}
      aria-disabled={entry.disabled || undefined}
      aria-checked={checkable ? Boolean(entry.checked) : undefined}
      aria-haspopup={hasSubmenu ? "true" : undefined}
      aria-expanded={hasSubmenu ? submenuOpen : undefined}
      className="lumen-menu__item"
      onClick={entry.disabled ? undefined : onClick}
    >
      <span className="lumen-menu__check" aria-hidden="true" />
      <span className="lumen-menu__label">{resolveLabel(entry.labelKey)}</span>
      {entry.accelerator !== undefined && (
        <span className="lumen-menu__accelerator">{entry.accelerator}</span>
      )}
      {hasSubmenu && <span className="lumen-menu__submenu-arrow" aria-hidden="true" />}
    </div>
  );
}
