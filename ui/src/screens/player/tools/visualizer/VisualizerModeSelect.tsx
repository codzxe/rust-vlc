// w26.c3; w26.k10 (j10 P2-4) — VisualizerModeSelect: the "Audio
// Visualizer.svg" mode dropdown. The lane-scoped re-implementation of the
// Dropdown grammar (230-line combobox + ~160 CSS lines) is GONE — this is
// now a thin composition over the shared Dropdown primitive:
//
//   * the primitive carries the full combobox interaction model (listbox +
//     roving tabindex, arrows/Home/End, Enter/Space, Escape/Tab with focus
//     restoration, outside-pointer dismiss, typeahead, scrollport cap,
//     measured open-direction flip, the h14 defect-4 whole-list reveal —
//     w26.k10 moved that fold-fix into the primitive for every host);
//   * the frame's two-tone 20px mode previews ride the primitive's
//     additive `iconNode` option field (landed by this lane in
//     Dropdown.tsx — the ask pending since w26.c1's log): option rows lead
//     with the icon and the trigger mirrors the selected mode's;
//   * the frame's A7 tool-scale geometry (two-layer 48px trigger, 44px rows
//     @ 46 pitch, two-tone inks, 13px icon→label gap) is scoped onto the
//     primitive in visualizer.css — exactly the `.eq-tool__presets`
//     composition pattern the EQ presets selector uses.
//
// GEOMETRY (frame, "Audio Visualizer.svg" / "EQ presets.svg" family) lives
// in visualizer.css now; the five options + their icons stay in types.ts.
// Typeahead — the primitive's — comes for free (five options; previously
// the one omitted behavior, noted in the w26.c3 lane log).

import type { ReactElement } from "react";
import { Dropdown } from "../../../../components/menu";
import type { DropdownOption } from "../../../../components/menu";
import { VISUALIZER_MODES } from "./types";
import type { VisualizerLabelKey, VisualizerMode } from "./types";

/** Public props (mountable; state lives in the parent tool — frozen API). */
export interface VisualizerModeSelectProps {
  /** Selected mode id. */
  value: VisualizerMode;
  /** Commit seam — fires with the picked mode. */
  onChange: (mode: VisualizerMode) => void;
  /** Accessible name for the trigger + listbox (already resolved). */
  label: string;
  /** Resolves option label keys (the useVisualizerMsg seam). */
  resolveLabel: (key: VisualizerLabelKey) => string;
  /** Stable id for the root; generated when omitted. */
  id?: string;
  /** Extra classes on the root. */
  className?: string;
}

/**
 * VisualizerModeSelect — the frame's five-mode dropdown as a composition
 * over the shared Dropdown (see module header). The `viz-select` class is
 * the CSS scope visualizer.css keys its A7 grammar on.
 */
export function VisualizerModeSelect({
  value,
  onChange,
  label,
  resolveLabel,
  id,
  className = "",
}: VisualizerModeSelectProps): ReactElement {
  const options: DropdownOption<VisualizerLabelKey>[] = VISUALIZER_MODES.map(
    (mode) => ({
      value: mode.id,
      labelKey: mode.labelKey,
      iconNode: mode.icon === undefined ? undefined : <mode.icon />,
      // Structural option marker (the primitive renders it as data-mode) —
      // the tests key the selected row's identity on it.
      data: { mode: mode.id },
    }),
  );

  return (
    <Dropdown
      className={["viz-select", className].filter(Boolean).join(" ")}
      options={options}
      value={value}
      onChange={(next) => {
        onChange(next as VisualizerMode);
      }}
      label={label}
      resolveLabel={resolveLabel}
      id={id}
    />
  );
}
