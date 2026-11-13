import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import { useSwitchLabels } from "./useSwitchLabels";
import "./button.css";

/** The shared public contract of the two switch exports (j01-3, Lead
 * ruling: both stay — the button family keeps `Toggle` for the
 * control-parts lane, the form family keeps `Switch` for settings rows —
 * but ONE core implementation now backs them, so interaction/i18n/skin
 * fixes land once instead of twice). */
export interface SwitchCoreProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onChange"> {
  /** Controlled on/off state. */
  checked: boolean;
  /** Change handler — fires on click AND keyboard (Space/Enter via native button). */
  onChange: (checked: boolean) => void;
}

/**
 * Per-family class grammar for the three anatomy parts of the Toggle.svg
 * frame (68×32 rx6 track): the 33×26 ON micro-label segment, the 26×26
 * knob, the 33×26 OFF micro-label segment. Screens' CSS keeps hooks on the
 * form family's `.lumen-switch*` set, so the two class grammars both ship —
 * `button.css` styles them through shared selector lists (one rule, both
 * sets), which is what keeps the skins from diverging again (j01-9).
 */
export interface SwitchSkin {
  /** Root class (composed with the consumer `className`). */
  root: string;
  /** 33×26 ON segment class(es) — carries the localized ON micro label. */
  onSegment: string;
  /** 26×26 knob class. */
  knob: string;
  /** 33×26 OFF segment class(es) — carries the localized OFF micro label. */
  offSegment: string;
}

/**
 * SwitchCore — the single implementation of the W26 Toggle.svg switch
 * (doc 08 §1.5, §4.5): a native `<button>` (Space/Enter for free) carrying
 * `role="switch"` + `aria-checked` — a switch carries checked state, not
 * pressed. Anatomy per Toggle.svg: ON = accent-tinted 33×26 segment ("ON"
 * in accent) + 26×26 accent-strong knob right; OFF = 26×26 stroke-border
 * knob left + panel 33×26 segment ("OFF" secondary, the 4.F4 contrast
 * fix). The micro labels are decorative (aria-hidden — state rides on
 * aria-checked) but user-visible, so they resolve through useSwitchLabels()
 * (i18n seam, j01-4). The two exported names are thin skins over this core:
 * `button/Toggle` (`.lumen-toggle*`) and `form/Switch` (`.lumen-switch*`).
 */
export const SwitchCore = forwardRef<HTMLButtonElement, SwitchCoreProps & { skin: SwitchSkin }>(
  (
    { checked, onChange, disabled, type = "button", skin, className = "", onClick, ...rest },
    ref,
  ) => {
    const labels = useSwitchLabels();
    return (
      <button
        ref={ref}
        type={type}
        role="switch"
        aria-checked={checked}
        className={[skin.root, className].filter(Boolean).join(" ")}
        disabled={disabled}
        onClick={(event) => {
          if (disabled) return; // disabled means disabled — never fire
          onClick?.(event);
          onChange(!checked);
        }}
        {...rest}
      >
        <span className={skin.onSegment} aria-hidden="true">
          {labels.on}
        </span>
        <span className={skin.knob} aria-hidden="true" />
        <span className={skin.offSegment} aria-hidden="true">
          {labels.off}
        </span>
      </button>
    );
  },
);

SwitchCore.displayName = "SwitchCore";
