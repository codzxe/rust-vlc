import { forwardRef } from "react";
import type { HTMLAttributes } from "react";
import { Slider } from "./Slider";
import type { SliderProps } from "./Slider";

/**
 * Volume slider primitive (card 4.B2; doc 08 §1.5 volume.svg: "Volume flyout
 * — vertical slider, stroke-based, 129×645"). Anatomy in slider.css: 2px
 * stroke track, accent fill, 8×24 rx4 `accent-strong` thumb (EQ thumb color,
 * §1.4). The icon + flyout chrome (mute/low/med/high × hover states) is the
 * later `VolumeControl` composite (doc 08 §4.3) — this is just the slider.
 */
export interface VolumeSliderProps extends Omit<HTMLAttributes<HTMLDivElement>, "onChange"> {
  /** Engine-reported volume, 0…1 (doc 05 command layer range). */
  value: number;
  disabled?: boolean;
  /** Live volume during drag (visual only, doc 08 §5). */
  onChangeLive?: (value: number) => void;
  /** Fires on release / keyboard set — the value callers act on. */
  onCommit?: (value: number) => void;
}

const formatPercent = (value: number): string => `${Math.round(value * 100)}%`;

export const VolumeSlider = forwardRef<HTMLDivElement, VolumeSliderProps>(
  function VolumeSlider({ value, disabled = false, onChangeLive, onCommit, ...rest }, ref) {
    const sliderProps: SliderProps = {
      value,
      min: 0,
      max: 1,
      step: 0.01,
      orientation: "vertical",
      disabled,
      onChangeLive,
      onCommit,
      formatValue: formatPercent,
      className: "lumen-volume",
    };
    return (
      // aria-label comes from the caller (i18n `useMsg`, doc 07 §6.5) via rest
      // props — no user-visible literals in primitives.
      <Slider ref={ref} {...sliderProps} {...rest} />
    );
  },
);

VolumeSlider.displayName = "VolumeSlider";
