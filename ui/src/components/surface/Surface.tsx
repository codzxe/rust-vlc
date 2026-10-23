import { useEffect, useId, useRef } from "react";
import type { ReactNode, RefObject } from "react";
import "./surface.css";

/** Where a Sheet anchors to the viewport edge (doc 08 §4: Sheet = edge-anchored dialog). */
export type SheetSide = "top" | "right" | "bottom" | "left";

/** Reasons a surface can request close (doc 08 §4.5: `Esc` closes overlays). */
export type SurfaceCloseReason = "escape" | "outside";

/**
 * Elements considered focusable for the trap. Disabled controls are excluded
 * at query time; hidden ones at cycle time (offsetParent check).
 */
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

export interface SurfaceOverlayProps {
  /** Whether the dialog/sheet is mounted and trapped. */
  open: boolean;
  /**
   * Called when the user requests dismissal: Escape always; outside click only
   * when `closeOnOutsideClick` (destructive forms opt out — doc 08 §4.5).
   */
  onClose: (reason: SurfaceCloseReason) => void;
  /** Element id labelling the surface (preferred accessible name). */
  labelledBy?: string;
  /** Accessible name when no title element id is supplied. */
  label?: string;
  /** Allow scrim clicks to close. @default false */
  closeOnOutsideClick?: boolean;
  children: ReactNode;
}

/**
 * Focus trap shared by Dialog and Sheet (doc 08 §4.5: `role="dialog"` + focus
 * trap; `Esc` closes). On open: the opener is remembered, focus moves to the
 * first focusable descendant (else the surface itself, which has tabIndex=-1).
 * While open: Tab/Shift+Tab cycle inside the surface. On close: focus is
 * restored to the opener.
 */
function useSurfaceFocusTrap(
  open: boolean,
  onClose: (reason: SurfaceCloseReason) => void,
  ref: RefObject<HTMLElement | null>,
) {
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    if (!open) return;
    const node = ref.current;
    if (!node) return;

    // Remember the opener so focus can be restored when the surface closes.
    const opener = document.activeElement instanceof Element ? document.activeElement : null;
    // Move focus into the surface (first focusable, else the surface itself).
    const focusables = node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    (focusables[0] ?? node).focus();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onCloseRef.current("escape");
        return;
      }
      if (event.key !== "Tab") return;
      // The trap takes over Tab entirely (preventDefault + explicit move) so
      // cycling is deterministic in browsers *and* jsdom (which applies no
      // default Tab movement). Visibility check is structural because jsdom
      // has no layout (offsetParent is always null): skip `hidden` subtrees.
      event.preventDefault();
      const items = Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (el) => el.closest("[hidden]") === null,
      );
      if (items.length === 0) {
        node.focus(); // Surface itself keeps focus (tabIndex=-1).
        return;
      }
      const active = document.activeElement;
      const currentIndex = active instanceof HTMLElement ? items.indexOf(active) : -1;
      const nextIndex = event.shiftKey
        ? currentIndex <= 0
          ? items.length - 1
          : currentIndex - 1
        : currentIndex === -1 || currentIndex === items.length - 1
          ? 0
          : currentIndex + 1;
      items[nextIndex]!.focus();
    };

    node.addEventListener("keydown", onKeyDown);
    return () => {
      node.removeEventListener("keydown", onKeyDown);
      if (opener instanceof HTMLElement) opener.focus();
    };
  }, [open, ref]);
}

/** Shared scrim + dialog semantics markup for Dialog and Sheet. */
function useSurfaceA11y(labelledBy?: string, label?: string) {
  const generated = useId();
  // Prefer an explicit title element; else synthesize one from `label` so the
  // surface always exposes aria-labelledby (doc 08 §4.5).
  const id = labelledBy ?? generated;
  return { titleId: id, ariaLabel: labelledBy ? undefined : (label ?? "") };
}

/**
 * Dialog — modal surface (doc 08 §4.5: `role="dialog"` + focus trap; §1.4:
 * settings window rx14 `#0E0E0E` → surface-panel + radius-window; scrim at
 * --opacity-scrim). Escape closes; focus restores to the opener.
 */
export function Dialog({
  open,
  onClose,
  labelledBy,
  label,
  closeOnOutsideClick = false,
  children,
  className = "",
}: SurfaceOverlayProps & { className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useSurfaceFocusTrap(open, onClose, ref);
  const { titleId, ariaLabel } = useSurfaceA11y(labelledBy, label);

  if (!open) return null;
  return (
    <div className="lumen-scrim" onClick={closeOnOutsideClick ? () => onClose("outside") : undefined}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-label={ariaLabel || undefined}
        tabIndex={-1}
        className={`lumen-dialog ${className}`.trim()}
        onClick={(e) => e.stopPropagation()}
      >
        {labelledBy ? null : (
          <span id={titleId} className="lumen-surface__sr-title">
            {label ?? ""}
          </span>
        )}
        {children}
      </div>
    </div>
  );
}

/**
 * Sheet — Dialog variant anchored to a viewport edge (doc 08 §4). Identical
 * trap + semantics; `side` picks the edge; the slide-in transition honors
 * `prefers-reduced-motion` (doc 08 §4.5).
 */
export function Sheet({
  open,
  onClose,
  labelledBy,
  label,
  closeOnOutsideClick = false,
  side = "right",
  children,
  className = "",
}: SurfaceOverlayProps & { side?: SheetSide; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useSurfaceFocusTrap(open, onClose, ref);
  const { titleId, ariaLabel } = useSurfaceA11y(labelledBy, label);

  if (!open) return null;
  return (
    <div className="lumen-scrim" onClick={closeOnOutsideClick ? () => onClose("outside") : undefined}>
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-label={ariaLabel || undefined}
        tabIndex={-1}
        className={`lumen-sheet lumen-sheet--${side} ${className}`.trim()}
        onClick={(e) => e.stopPropagation()}
      >
        {labelledBy ? null : (
          <span id={titleId} className="lumen-surface__sr-title">
            {label ?? ""}
          </span>
        )}
        {children}
      </div>
    </div>
  );
}
