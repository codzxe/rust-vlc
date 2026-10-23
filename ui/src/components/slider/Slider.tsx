import { forwardRef } from "react";
import type { CSSProperties, HTMLAttributes } from "react";
import "./slider.css";
import { toPercent, useSliderInteraction } from "./interaction";

/** Slider orientation (doc 08 §1.4: EQ + volume flyout are vertical). */
export type SliderOrientation = "horizontal" | "vertical";

export interface SliderProps extends Omit<HTMLAttributes<HTMLDivElement>, "onChange"> {
  /**
   * Current committed value. The slider is fully controlled: after a commit it
   * renders exactly what the caller passes back — never an optimistic jump
   * (doc 08 §5 truthfulness).
   */
  value: number;
  /** @default 0 */
  min?: number;
  /** @default 100 */
  max?: number;
  /** @default 1 */
  step?: number;
  /** Rendered along this axis (`ArrowUp` always increases). @default "horizontal" */
  orientation?: SliderOrientation;
  /**
   * Disabled: `aria-disabled` + token opacity, fully inert (no pointer,
   * no keyboard, no focus).
   */
  disabled?: boolean;
  /**
   * Live value during pointer drag and on each keyboard step — visual only
   * (doc 08 §5: fires continuously while scrubbing).
   */
  onChangeLive?: (value: number) => void;
  /**
   * Fires on pointer release and on keyboard set — the value callers act on.
   */
  onCommit?: (value: number) => void;
  /** Formats `aria-valuetext` (e.g. time or percent strings). */
  formatValue?: (value: number) => string;
}

/**
 * Generic slider primitive (doc 08 §4.2 primitives/Slider). Anatomy per doc 08
 * §1.4 miniplayer/EQ exports: 16px track rx6 `surface-deep`; fill inset 2px
 * `accent-tinted` rx4; 8×12 (horizontal) / 8×24 (vertical) thumb rx4.
 * Keyboard: arrows ±step (Shift = coarse ×10), PageUp/PageDown coarse,
 * Home/End min/max (doc 08 §4.5). All states token-styled in slider.css.
 */
export const Slider = forwardRef<HTMLDivElement, SliderProps>(function Slider(
  {
    value,
    min = 0,
    max = 100,
    step = 1,
    orientation = "horizontal",
    disabled = false,
    onChangeLive,
    onCommit,
    formatValue,
    className = "",
    ...rest
  },
  ref,
) {
  const { dragValue, handlers } = useSliderInteraction({
    value,
    min,
    max,
    step,
    orientation,
    disabled,
    onChangeLive,
    onCommit,
  });

  // While scrubbing, the preview drives both fill and thumb (visual only).
  const shown = dragValue ?? value;
  const percent = toPercent(shown, min, max);

  return (
    <div
      ref={ref}
      role="slider"
      className={["lumen-slider", orientation === "vertical" && "lumen-slider--vertical", className]
        .filter(Boolean)
        .join(" ")}
      tabIndex={disabled ? -1 : 0}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={shown}
      aria-valuetext={formatValue?.(shown)}
      aria-orientation={orientation}
      aria-disabled={disabled || undefined}
      data-dragging={dragValue !== null || undefined}
      style={
        {
          "--lumen-slider-fill": percent,
          "--lumen-slider-thumb": percent,
        } as CSSProperties
      }
      {...handlers}
      {...rest}
    >
      <div className="lumen-slider__track">
        <div className="lumen-slider__fill" />
        <div className="lumen-slider__thumb" />
      </div>
    </div>
  );
});

Slider.displayName = "Slider";
