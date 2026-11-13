import { forwardRef } from "react";
import { SwitchCore } from "../button/switchCore";
import type { SwitchCoreProps, SwitchSkin } from "../button/switchCore";

export interface SwitchProps extends SwitchCoreProps {
  /** Controlled on/off state. */
  checked: boolean;
  /** Change handler — fires on click AND keyboard (Space/Enter via native button). */
  onChange: (checked: boolean) => void;
}

/** The form family's class grammar for the shared switch core (j01-3):
 * settings/onboarding row CSS keeps its `.lumen-switch*` hooks (settings.css,
 * effects.css, subtitles.css), so the class set is frozen even though the
 * implementation is now the button family's core. */
const switchSkin: SwitchSkin = {
  root: "lumen-switch",
  onSegment: "lumen-switch__text lumen-switch__text--on",
  knob: "lumen-switch__fill",
  offSegment: "lumen-switch__text lumen-switch__text--off",
};

/**
 * Switch — the form family's export of the W26 Toggle.svg control (the
 * settings rows consume it through this module). Same public contract as
 * ever: `role="switch"` + `aria-checked`, onChange on click/keyboard, native
 * disabled gating. Pre-W26 this re-exported the button-family Toggle; the
 * w26.a06 wave shipped a local port (documented as temporary); w26.k01
 * (j01-3, Lead ruling) collapses the dual implementations back to ONE core
 * (`button/switchCore.tsx`) with this `.lumen-switch*` skin on top — the
 * two exported names stay (frozen public API), the implementation and the
 * CSS skin are shared, so fixes land once (j01-9's OFF-ink divergence was
 * the last one).
 *
 * Geometry is an honest port of Toggle.svg (see button.css): 68×32 rx6 track;
 * ON = accent-tinted 33×26 text segment ("ON" in accent) + accent-strong
 * 26×26 fill right; OFF = stroke-border fill left + panel text segment
 * ("OFF" secondary, 4.F4). The ON/OFF micro labels are decorative
 * (aria-hidden) but localized (useSwitchLabels → i18n keys, j01-4) — the
 * checked state is conveyed by aria-checked.
 */
export const Switch = forwardRef<HTMLButtonElement, SwitchProps>((props, ref) => (
  <SwitchCore ref={ref} skin={switchSkin} {...props} />
));

Switch.displayName = "Switch";
