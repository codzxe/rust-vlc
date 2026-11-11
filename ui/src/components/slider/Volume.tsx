import { forwardRef } from "react";
import type { HTMLAttributes } from "react";
import { Slider } from "./Slider";
import type { SliderProps } from "./Slider";

/**
 * Volume slider primitive (w26.a04 restyle; volume.svg + the Player.svg
 * volume group). Compact HORIZONTAL slider in the seekbar's slim grammar
 * (4px pill-ended track on the surface-deep well, circular thumb revealed
 * on hover/drag) — the level fill resolves to the frame-measured
 * green→amber→red signal gradient through the --gradient-volume-level token
 * when present, else the accent (slider.css). The mute icon + flyout chrome
 * remains the consumer composite's concern — this is just the slider.
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
  function VolumeSlider(
    { value, disabled = false, onChangeLive, onCommit, className = "", ...rest },
    ref,
  ) {
    // The `lumen-volume` grammar class is MERGED with the caller's className —
    // never replaced. (w26.a04 fix: a consumer className like the ControlBar's
    // `lumen-controls__volume-slider` used to clobber it via the rest spread,
    // silently dropping the slim compact grammar and leaving the base EQ pill
    // anatomy on the player bar.)
    const sliderProps: SliderProps = {
      value,
      min: 0,
      max: 1,
      step: 0.01,
      orientation: "horizontal",
      disabled,
      onChangeLive,
      onCommit,
      formatValue: formatPercent,
      className: ["lumen-volume", className].filter(Boolean).join(" "),
    };
    return (
      // aria-label comes from the caller (i18n `useMsg`, doc 07 §6.5) via rest
      // props — no user-visible literals in primitives.
      <Slider ref={ref} {...sliderProps} {...rest} />
    );
  },
);

VolumeSlider.displayName = "VolumeSlider";
