import { useEffect, useId, useRef, useState } from "react";
import type { ReactElement } from "react";
import "./menu.css";

/**
 * Tooltip show/hide hover-intent delays. Doc 08 pins no values —
 * [PROPOSAL] defaults recorded in states.md. Keyboard focus shows
 * immediately (WAI-ARIA APG: never delay for keyboard users).
 */
export const TOOLTIP_SHOW_DELAY_MS = 500;
export const TOOLTIP_HIDE_DELAY_MS = 150;

export interface TooltipProps {
  /** Tooltip text — caller resolves via useMsg() (i18n seam). */
  content: string;
  children: ReactElement;
  /** Hover-intent show delay (ms). @default 500 */
  showDelay?: number;
  /** Hover-intent hide delay (ms). @default 150 */
  hideDelay?: number;
  /** Preferred side of the trigger. @default "top" */
  side?: "top" | "bottom";
  id?: string;
}

/**
 * Tooltip (doc 08 §259 primitives list) — wraps a single trigger element in
 * a positioning span, shows a `role="tooltip"` bubble after a hover-intent
 * delay; focus/blur show/hide immediately; Escape hides. The wrapper carries
 * `aria-describedby` while open, linking trigger to bubble.
 */
export function Tooltip({
  content,
  children,
  showDelay = TOOLTIP_SHOW_DELAY_MS,
  hideDelay = TOOLTIP_HIDE_DELAY_MS,
  side = "top",
  id,
}: TooltipProps): ReactElement {
  const autoId = useId();
  const tooltipId = id ?? `lumen-tooltip-${autoId}`;
  const showTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [open, setOpen] = useState(false);

  const clearTimers = (): void => {
    if (showTimer.current !== null) clearTimeout(showTimer.current);
    if (hideTimer.current !== null) clearTimeout(hideTimer.current);
    showTimer.current = null;
    hideTimer.current = null;
  };

  useEffect(() => clearTimers, []);

  const hide = (): void => {
    clearTimers();
    setOpen(false);
  };

  return (
    <span
      className="lumen-tooltip"
      aria-describedby={open ? tooltipId : undefined}
      onMouseEnter={() => {
        if (hideTimer.current !== null) {
          clearTimeout(hideTimer.current);
          hideTimer.current = null;
        }
        if (showTimer.current !== null) clearTimeout(showTimer.current);
        showTimer.current = setTimeout(() => {
          showTimer.current = null;
          setOpen(true);
        }, showDelay);
      }}
      onMouseLeave={() => {
        if (showTimer.current !== null) {
          clearTimeout(showTimer.current);
          showTimer.current = null;
        }
        if (!open) return;
        if (hideTimer.current !== null) clearTimeout(hideTimer.current);
        hideTimer.current = setTimeout(() => {
          hideTimer.current = null;
          setOpen(false);
        }, hideDelay);
      }}
      onFocus={() => setOpen(true)}
      onBlur={hide}
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) hide();
      }}
    >
      {children}
      {open && (
        <span
          id={tooltipId}
          role="tooltip"
          className={["lumen-tooltip__bubble", side === "bottom" && "lumen-tooltip__bubble--bottom"]
            .filter(Boolean)
            .join(" ")}
        >
          {content}
        </span>
      )}
    </span>
  );
}
