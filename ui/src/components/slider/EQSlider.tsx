import { forwardRef } from "react";
import { Slider } from "./Slider";
import type { SliderProps } from "./Slider";

/**
 * EQ band slider (w26.a04, additive export) — the EQ pill slider per EQ.png
 * and W25 spec §2: full-width 14px r7 pill track on the surface-deep well,
 * amber gradient fill (accent.tinted → eq fill end, slider.css) and an
 * 8×20 r4 amber handle.
 *
 * The VALUE CHIP (audio value.svg: r6 dark well + value text) is rendered by
 * CONSUMERS, outside this slider — EQ rows compose label + chip + EQSlider.
 *
 * Interaction contract is the shared slider engine (doc 08 §5): arrows ±step
 * (Shift = coarse ×10), PageUp/PageDown coarse, Home/End bounds,
 * live-then-commit, fully controlled.
 */
export type EQSliderProps = Omit<SliderProps, "orientation">;

export const EQSlider = forwardRef<HTMLDivElement, EQSliderProps>(function EQSlider(
  { className = "", ...rest },
  ref,
) {
  // aria-label comes from the caller (i18n `useMsg`, doc 07 §6.5) via rest
  // props — no user-visible literals in primitives.
  return (
    <Slider
      ref={ref}
      orientation="horizontal"
      className={["lumen-eq-slider", className].filter(Boolean).join(" ")}
      {...rest}
    />
  );
});

EQSlider.displayName = "EQSlider";
