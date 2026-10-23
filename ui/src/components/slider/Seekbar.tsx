import { forwardRef } from "react";
import type { CSSProperties, HTMLAttributes } from "react";
import { clamp, toPercent, useSliderInteraction } from "./interaction";
import "./slider.css";

/**
 * An engine-reported buffered range in seconds (doc 08 §4.3 SeekBar).
 * Rendered as an honest sub-region overlay — the caller passes engine data;
 * the component never invents buffer progress.
 */
export interface SeekbarBufferRange {
  start: number;
  end: number;
}

export interface SeekbarProps extends Omit<HTMLAttributes<HTMLDivElement>, "onChange"> {
  /**
   * Engine-reported playback position in seconds (doc 08 §5.2 — the UI never
   * predicts or caches-past engine state).
   */
  value: number;
  /** Total media duration in seconds. `<= 0` renders an inert, disabled bar. */
  duration: number;
  /** Engine-reported buffer ranges in seconds, rendered honestly. */
  buffered?: readonly SeekbarBufferRange[];
  /**
   * Outstanding requested seek target in seconds (doc 08 §5.2.2): after
   * commit, the knob locks at the requested position — styled distinctly —
   * while the fill stays at the engine position until the caller confirms.
   * Caller-owned; the component never fakes a pending state.
   */
  pending?: number;
  disabled?: boolean;
  /** Live scrub position in seconds (visual only, doc 08 §5.2.1). */
  onChangeLive?: (seconds: number) => void;
  /** Fires on pointer release / keyboard set with the target seconds. */
  onCommit?: (seconds: number) => void;
  /** Formats `aria-valuetext` (doc 08 §4.5: formatted time). */
  formatTime?: (seconds: number) => string;
}

/** Default time formatter: `M:SS` / `H:MM:SS`, negative-safe. */
export const formatSeekTime = (seconds: number): string => {
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m);
  return `${h > 0 ? `${h}:` : ""}${mm}:${String(s).padStart(2, "0")}`;
};

/**
 * Seekbar (doc 08 §4.3, §5.2). Anatomy per design (§1.4): track 28px rx6
 * `surface-deep`; progress fill inset 2px (24px) rx4 `accent-tinted`; handle
 * 8×24 rx4 `accent` — a slim vertical bar, not a circle.
 *
 * Truthful pending-seek rendering (§5.2):
 * 1. Scrub: knob follows the local preview; fill stays at the engine position.
 * 2. Commit: caller dispatches `playback.seek` and passes `pending`; knob
 *    locks at the target (`data-pending`, accent-strong), fill still engine.
 * 3. Confirmation: caller updates `value` and clears `pending` — both animate
 *    to engine truth. No fabricated jumps anywhere.
 */
export const Seekbar = forwardRef<HTMLDivElement, SeekbarProps>(function Seekbar(
  {
    value,
    duration,
    buffered,
    pending,
    disabled = false,
    onChangeLive,
    onCommit,
    formatTime = formatSeekTime,
    className = "",
    ...rest
  },
  ref,
) {
  const inert = disabled || duration <= 0;
  const { dragValue, handlers } = useSliderInteraction({
    value,
    min: 0,
    max: duration,
    step: Math.max(duration / 1000, 0.001),
    orientation: "horizontal",
    disabled: inert,
    onChangeLive,
    onCommit,
  });

  // Knob: scrub preview → pending request → engine truth.
  const knob = dragValue ?? pending ?? value;
  const thumbPercent = toPercent(knob, 0, duration);
  // Fill: always the engine-reported position — never the request.
  const fillPercent = toPercent(value, 0, duration);

  return (
    <div
      ref={ref}
      role="slider"
      className={["lumen-slider", "lumen-slider--horizontal", "lumen-seekbar", className]
        .filter(Boolean)
        .join(" ")}
      tabIndex={inert ? -1 : 0}
      // aria-label comes from the caller (i18n `useMsg`, doc 07 §6.5) via rest
      // props — no user-visible literals in primitives.
      aria-valuemin={0}
      aria-valuemax={duration > 0 ? duration : 0}
      aria-valuenow={knob}
      aria-valuetext={formatTime(knob)}
      aria-orientation="horizontal"
      aria-disabled={inert || undefined}
      data-dragging={dragValue !== null || undefined}
      data-pending={dragValue === null && pending !== undefined ? true : undefined}
      style={
        {
          "--lumen-slider-fill": fillPercent,
          "--lumen-slider-thumb": thumbPercent,
        } as CSSProperties
      }
      {...handlers}
      {...rest}
    >
      <div className="lumen-slider__track">
        {buffered?.map((range, index) => {
          const start = clamp((range.start / duration) * 100, 0, 100);
          const width = clamp(((range.end - range.start) / duration) * 100, 0, 100 - start);
          return (
            <div
              key={`${range.start}-${range.end}-${index}`}
              className="lumen-seekbar__buffer"
              data-range-start={range.start}
              data-range-end={range.end}
              style={{ left: `${start}%`, width: `${width}%` }}
            />
          );
        })}
        <div className="lumen-slider__fill" />
        <div className="lumen-slider__thumb" />
      </div>
    </div>
  );
});

Seekbar.displayName = "Seekbar";
