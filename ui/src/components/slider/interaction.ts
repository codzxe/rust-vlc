import { useCallback, useRef, useState } from "react";
import type {
  KeyboardEvent as ReactKeyboardEvent,
  PointerEvent as ReactPointerEvent,
} from "react";

/**
 * Slider primitives (card 4.B2). Design provenance: docs/08-ui-figma-fidelity.md
 * §1.4 ("Miniplayer seek 375×16 rx6, knob 8×12 rx4"; "Slider (EQ, vertical)
 * track 16, fill 12, thumb 8×24 rx4 #AB6314"), §4 (component architecture),
 * §5 (live-vs-committed / truthful pending semantics), §4.5 (ARIA: slider role
 * with aria-valuenow/min/max/text).
 *
 * Interaction contract (doc 08 §5, KEY SEMANTICS):
 * - `onChangeLive` fires continuously while the user drags (visual only).
 * - `onCommit` fires on pointer release and on keyboard set — the value
 *   callers act on. The component is fully controlled: after commit it renders
 *   whatever the caller passes back via `value` (engine truth), never an
 *   optimistic jump.
 */

/** Clamp `value` into `[min, max]`. */
export const clamp = (value: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, value));

/** Snap `value` onto the `min + n·step` lattice (no-op for step ≤ 0). */
export const quantize = (value: number, min: number, step: number): number => {
  if (!Number.isFinite(step) || step <= 0) return value;
  return min + Math.round((value - min) / step) * step;
};

/** Map a pointer event to a step-quantized value, honoring orientation. */
const valueFromPointer = (
  event: ReactPointerEvent<HTMLElement>,
  min: number,
  max: number,
  step: number,
  orientation: "horizontal" | "vertical",
): number => {
  const rect = event.currentTarget.getBoundingClientRect();
  const range = max - min;
  const along =
    orientation === "horizontal"
      ? (event.clientX - rect.left) / (rect.width || 1)
      : (rect.bottom - event.clientY) / (rect.height || 1);
  return clamp(quantize(min + clamp(along, 0, 1) * range, min, step), min, max);
};

export interface UseSliderInteractionArgs {
  value: number;
  min: number;
  max: number;
  step: number;
  orientation: "horizontal" | "vertical";
  disabled: boolean;
  onChangeLive?: (value: number) => void;
  onCommit?: (value: number) => void;
}

/**
 * Shared pointer/keyboard interaction engine for Slider and Seekbar.
 * Exposes `dragValue` (local preview while scrubbing, `null` at rest) and the
 * DOM handlers to spread on the `role="slider"` element. Pointer capture is
 * feature-detected (jsdom has no active-pointer table).
 */
export function useSliderInteraction({
  value,
  min,
  max,
  step,
  orientation,
  disabled,
  onChangeLive,
  onCommit,
}: UseSliderInteractionArgs) {
  /** Local preview position while scrubbing (doc 08 §5.2.1). */
  const [dragValue, setDragValue] = useState<number | null>(null);
  const draggingRef = useRef(false);
  /** Last value emitted via onChangeLive in the current gesture. */
  const emittedRef = useRef<number | null>(null);

  const emitLive = useCallback(
    (next: number) => {
      if (emittedRef.current !== next) {
        emittedRef.current = next;
        onChangeLive?.(next);
      }
    },
    [onChangeLive],
  );

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (disabled) return;
      event.currentTarget.focus();
      try {
        event.currentTarget.setPointerCapture?.(event.pointerId);
      } catch {
        /* jsdom: no active pointers — capture is best-effort */
      }
      draggingRef.current = true;
      emittedRef.current = null;
      const next = valueFromPointer(event, min, max, step, orientation);
      setDragValue(next);
      emitLive(next);
    },
    [disabled, emitLive, min, max, step, orientation],
  );

  const onPointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (!draggingRef.current || disabled) return;
      const next = valueFromPointer(event, min, max, step, orientation);
      setDragValue(next);
      emitLive(next);
    },
    [disabled, emitLive, min, max, step, orientation],
  );

  const endDrag = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>, commit: boolean) => {
      if (!draggingRef.current) return;
      draggingRef.current = false;
      try {
        event.currentTarget.releasePointerCapture?.(event.pointerId);
      } catch {
        /* best-effort */
      }
      if (commit) onCommit?.(emittedRef.current ?? value);
      setDragValue(null);
      emittedRef.current = null;
    },
    [onCommit, value],
  );

  const onPointerUp = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => endDrag(event, true),
    [endDrag],
  );

  /** A cancelled gesture (e.g. pointercancel) aborts without commit. */
  const onPointerCancel = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => endDrag(event, false),
    [endDrag],
  );

  const onKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      if (disabled) return;
      const coarse = step * 10;
      let next: number;
      switch (event.key) {
        case "ArrowRight":
        case "ArrowUp":
          next = value + (event.shiftKey ? coarse : step);
          break;
        case "ArrowLeft":
        case "ArrowDown":
          next = value - (event.shiftKey ? coarse : step);
          break;
        case "PageUp":
          next = value + coarse;
          break;
        case "PageDown":
          next = value - coarse;
          break;
        case "Home":
          next = min;
          break;
        case "End":
          next = max;
          break;
        default:
          return;
      }
      event.preventDefault();
      const clamped = clamp(quantize(next, min, step), min, max);
      if (clamped === value) return;
      // Keyboard set: live (visual trackers) then commit (the value callers
      // act on) — call order is part of the contract (doc 08 §5).
      emitLive(clamped);
      onCommit?.(clamped);
    },
    [disabled, emitLive, onCommit, value, min, max, step],
  );

  return {
    dragValue,
    handlers: { onPointerDown, onPointerMove, onPointerUp, onPointerCancel, onKeyDown },
  };
}

/** Percent (0–100) of `value` within `[min, max]`, clamped. */
export const toPercent = (value: number, min: number, max: number): number => {
  const range = max - min;
  if (!Number.isFinite(range) || range <= 0) return 0;
  return clamp(((value - min) / range) * 100, 0, 100);
};
