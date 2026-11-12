// w26.c3 — visualizer tool shared types (ui/src/screens/player/tools/visualizer).
// The tool = the collapsible tool-card grammar (w26.a20 §D1) hosting the
// "Audio Visualizer.svg" mode dropdown (§A7) plus a levels-driven display:
//
//   * mode selector — the a19 A9 Dropdown grammar the A7 frame reuses
//     verbatim (362×48 rx6 #090909 trigger + 358×44 rx4 #191919 field,
//     open list rx6 #191919 + shadow, 44px rows @ 46 pitch, selected
//     #1F1F1F + orange check) with the frame's five options and their
//     two-tone 20px leading icons;
//   * display — renders the selected mode from a `levels` array (one entry
//     per bar, normalized 0..1). HONEST RENDERING: no FFT/levels source
//     exists yet, so absent levels render the frame's RESTING depiction of
//     the mode (static, never animated, never simulated — no idle shimmer:
//     the frame carries none).
//
// Lane-scoped selector rationale: the shared Dropdown's `icon?: IconName`
// API cannot carry the frame's two-tone mode previews (custom nodes); the
// w26.c1 log flagged the additive ReactNode ask to the menu lane. Until it
// lands, the selector lives here with the primitive's exact grammar +
// interaction model (listbox, roving tabindex, arrows/Home/End,
// Enter/Space, Escape/Tab, outside-click dismiss).

import type { ReactElement } from "react";
import type { MessageKey } from "../../../../i18n/messages";
import type { keys as laneKeys } from "../../../../i18n/keys/w26.c3";
import type { keys as toolsKeys } from "../../../../i18n/keys/w26.a18";
import { ScopeIcon, SpectrometerIcon, SpectrumIcon, WaveformIcon } from "./ModeIcons";

/** Keys resolvable by `useVisualizerMsg` (own lane keys + reused a18 keys). */
export type VisualizerLabelKey =
  | MessageKey
  | keyof typeof laneKeys
  | keyof typeof toolsKeys;

/** The frame's five modes, in frame order ("Audio Visualizer.svg"). */
export type VisualizerMode =
  | "disabled"
  | "spectrometer"
  | "scope"
  | "spectrum"
  | "waveform";

/** One mode's definition (label key + the frame's two-tone leading icon). */
export interface VisualizerModeDef {
  /** Stable mode id (state + data-mode marker). */
  readonly id: VisualizerMode;
  /** Label key (frame: row label #8F8F8F ≈15px). */
  readonly labelKey: VisualizerLabelKey;
  /** The frame's two-tone 20px leading icon; omitted on Disabled (frame). */
  readonly icon?: (props: { className?: string }) => ReactElement;
}

/** The five frame modes ("Audio Visualizer.svg" open-list rows, in order). */
export const VISUALIZER_MODES: readonly VisualizerModeDef[] = [
  {
    id: "disabled",
    labelKey: "player.tools.visualizer.mode.disabled",
  },
  {
    id: "spectrometer",
    labelKey: "player.tools.visualizer.mode.spectrometer",
    icon: SpectrometerIcon,
  },
  {
    id: "scope",
    labelKey: "player.tools.visualizer.mode.scope",
    icon: ScopeIcon,
  },
  {
    id: "spectrum",
    labelKey: "player.tools.visualizer.mode.spectrum",
    icon: SpectrumIcon,
  },
  {
    id: "waveform",
    labelKey: "player.tools.visualizer.mode.waveform",
    icon: WaveformIcon,
  },
];

/**
 * The display's RESTING depiction of each bar mode — the frame's own icon
 * geometry read off "Audio Visualizer.svg" (Scope bars 5/14/12/16/13/8 from
 * the y215 baseline, Spectrum bars 5/14/12/16/13/8 from y261, Waveform
 * half-heights 2/7/5/9/6/4 around the y298 midline), normalized to each
 * icon's tallest bar. Static by construction: the honest no-data state,
 * never animated (the frame shows no idle motion).
 *
 * w26.i07 (h14 defect 5, g16-P3-4): spectrum bar 4 was transcribed 19
 * (frame Vector 87_2 = 16 — the same 5/14/12/16/13/8 set ModeIcons carries);
 * the wrong max skewed every other bar's normalization ~16% short.
 */
export const RESTING_LEVELS: Readonly<
  Record<"scope" | "spectrum" | "waveform", readonly number[]>
> = {
  scope: [5, 14, 12, 16, 13, 8],
  spectrum: [5, 14, 12, 16, 13, 8],
  waveform: [2, 7, 5, 9, 6, 4],
};

/** Normalizes a resting icon's raw heights to 0..1 levels. */
export function restingLevels(mode: "scope" | "spectrum" | "waveform"): number[] {
  const bars = RESTING_LEVELS[mode];
  const max = Math.max(...bars);
  return bars.map((height) => height / max);
}

/** Props the mountable tool accepts (uncontrolled, wire-ready). */
export interface VisualizerToolProps {
  /**
   * Initial expanded state (uncontrolled). Frames ship tool cards
   * COLLAPSED — `defaultOpen` exists for hosts (harness, effects area)
   * that want the card open on mount.
   * @default false
   */
  defaultOpen?: boolean;
  /**
   * Initially selected mode (uncontrolled). The frame's default is
   * "Disabled" — the resting tool shows the selector only, exactly the
   * frame's closed state.
   * @default "disabled"
   */
  defaultMode?: VisualizerMode;
  /**
   * Live band levels, normalized 0..1 (clamped), ONE ENTRY PER BAR in
   * low→high frequency order. Readonly — the host owns the array and
   * pushes a new one per animation frame; the component only renders it.
   *
   * HONEST RENDERING: no FFT/analysis source exists yet. When omitted
   * (or empty), the display renders the mode's RESTING state — the
   * frame's static depiction — never simulated or animated data.
   */
  levels?: readonly number[];
  /**
   * Fires on every committed mode change (option click / keyboard
   * select) — the wire seam for the effects settings area + the tools
   * dialog when the audio-effects command card lands.
   */
  onChange?: (mode: VisualizerMode) => void;
  /** Stable id for the card's aria wiring; generated when omitted. */
  id?: string;
  /** Extra classes on the card root. */
  className?: string;
}
