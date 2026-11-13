import { forwardRef } from "react";
import { SwitchCore } from "./switchCore";
import type { SwitchCoreProps, SwitchSkin } from "./switchCore";
import "./button.css";

export interface ToggleProps extends SwitchCoreProps {
  /** Controlled on/off state. */
  checked: boolean;
  /** Change handler — fires on click AND keyboard (Space/Enter via native button). */
  onChange: (checked: boolean) => void;
}

/** The button family's class grammar for the shared switch core (j01-3):
 * control-parts lane consumers keep their `.lumen-toggle*` hooks. */
const toggleSkin: SwitchSkin = {
  root: "lumen-toggle",
  onSegment: "lumen-toggle__fill",
  knob: "lumen-toggle__knob",
  offSegment: "lumen-toggle__off",
};

/**
 * Toggle switch primitive (doc 08 §1.5 Toggle.svg, §4.5: "toggles → switch").
 * Exposes `role="switch"` + `aria-checked` — the doc's ARIA contract (a
 * `switch` carries checked state, not `aria-pressed`; deviation from the
 * task-card blurb recorded in states.md). Geometry per Toggle.svg (W26):
 * 68x32 rx6 track on `--color-surface`; ON = 33x26 rx3 accent-tinted segment
 * ("ON" accent micro label) + 26x26 rx3 accent-strong knob right; OFF =
 * 26x26 rx3 stroke-border knob left + 33x26 rx3 surface-panel segment ("OFF"
 * micro label, content-secondary — the 4.F4 a11y fix). Micro labels (doc 08
 * §1.3 row 4.4, §1.5) are decorative (aria-hidden) but localized
 * (useSwitchLabels → i18n keys, j01-4): the switch's checked state is
 * conveyed by aria-checked. Implementation: the shared SwitchCore —
 * `form/Switch` is the same core in the `.lumen-switch*` grammar.
 */
export const Toggle = forwardRef<HTMLButtonElement, ToggleProps>((props, ref) => (
  <SwitchCore ref={ref} skin={toggleSkin} {...props} />
));

Toggle.displayName = "Toggle";
