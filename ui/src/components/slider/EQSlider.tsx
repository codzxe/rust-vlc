import { forwardRef } from "react";
import { Slider } from "./Slider";
import type { SliderOrientation, SliderProps } from "./Slider";

/**
 * EQ band slider (w26.a04, additive export; w26.f05 re-measure) — the EQ
 * slider per the audio-tool frames as SVG-measured (w26.a20 §A1, EQ.svg /
 * Audio - Equalizer.svg; slider.css carries the anatomy): 16px rx6 track on
 * the surface well (#090909), FLAT accent-tinted fill (#371F06, 12px rx4
 * inset 2) and an 8×24 rx4 accent-strong handle — horizontal in the tool
 * rows, `orientation="vertical"` in the A2 fader panel (24×8 handle).
 *
 * The VALUE CHIP (audio value.svg / EQ.svg's 60×32 readouts) is rendered by
 * CONSUMERS, outside this slider — EQ rows compose label + chip + EQSlider.
 *
 * Interaction contract is the shared slider engine (doc 08 §5): arrows ±step
 * (Shift = coarse ×10), PageUp/PageDown coarse, Home/End bounds,
 * live-then-commit, fully controlled.
 */
export type EQSliderProps = Omit<SliderProps, "orientation"> & {
  /**
   * Render axis — the classic EQ band view mounts vertical faders (EQ.svg's
   * rotate(-90) tracks). Optional and additive (w26.f05): callers that pass
   * nothing keep the horizontal rows exactly as before.
   * @default "horizontal"
   */
  orientation?: SliderOrientation;
};

export const EQSlider = forwardRef<HTMLDivElement, EQSliderProps>(function EQSlider(
  { className = "", orientation = "horizontal", ...rest },
  ref,
) {
  // aria-label comes from the caller (i18n `useMsg`, doc 07 §6.5) via rest
  // props — no user-visible literals in primitives.
  return (
    <Slider
      ref={ref}
      orientation={orientation}
      className={["lumen-eq-slider", className].filter(Boolean).join(" ")}
      {...rest}
    />
  );
});

EQSlider.displayName = "EQSlider";
