import { cloneElement, useEffect, useId, useRef, useState } from "react";
import type { ReactElement } from "react";
import { TOOLTIP_HIDE_DELAY_MS, TOOLTIP_SHOW_DELAY_MS } from "./timings";
import "./menu.css";

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
 * delay; focus/blur show/hide immediately; Escape hides. w26.k02 (j02 J02-1):
 * `aria-describedby` rides ON THE CHILD TRIGGER (cloneElement), not on this
 * non-focusable wrapper — SRs announce descriptions of the *focused*
 * element, and focus lands on the child, so a wrapper-level describedby was
 * never announced. The child must forward unknown props to its DOM node
 * (plain elements do); any describedby the child already carries is MERGED
 * ("existing tooltip-id") while open and preserved when closed. The wrapper
 * keeps the positioning + hover/focus/key event seams (events bubble).
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

  // w26.k02 (j02 J02-1): compose the trigger's describedby — the tooltip id
  // joins whatever the child already points at; closed state keeps the
  // child's own wiring untouched.
  const childDescribedBy = (() => {
    const value = (children.props as { "aria-describedby"?: unknown } | undefined)?.["aria-describedby"];
    return typeof value === "string" && value !== "" ? value : undefined;
  })();
  const describedBy = open ? [childDescribedBy, tooltipId].filter(Boolean).join(" ") : childDescribedBy;
  const trigger =
    describedBy === undefined
      ? children
      : cloneElement(children as ReactElement<{ "aria-describedby"?: string }>, {
          "aria-describedby": describedBy,
        });

  return (
    <span
      className="lumen-tooltip"
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
        if (event.key === "Escape" && open) {
          hide();
          /* w26.k02 (j02 J02-13): the tooltip was open and CONSUMED the
           * Escape — stop it bubbling so a tooltiped control inside a host
           * dialog no longer closes the dialog while merely hiding the
           * tooltip. Closed-state Escape passes through untouched. */
          event.stopPropagation();
        }
      }}
    >
      {trigger}
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
