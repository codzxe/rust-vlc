// w26.c2 — StereoWidenerTool (frame: "Audio - Stereo Widener.svg",
// 400×821): the 4-row stereo-widener card — collapsed 360×48 rx6 #090909 /
// expanded 360×362 rx6 #1B1B1B, header 59px, rows pitch 76, PLAIN values
// (percent units on the gain rows, "ms" on the delay row).
//
// Tick marks per frame (analysis §A6): Delay Time has NONE; Feedback 8,
// Crossfeed 7, Dry Mix 9 — each default lands on its tick (0.2 of 0.9 at
// 2/9, 0.3 of 0.8 at 3/8, 0.8 of 1.0 at 8/10). The gain ranges match the
// widener semantics the frame encodes (0–0.9 / 0–0.8 / 0–1.0).

import type { ReactElement } from "react";
import { DynamicsCard } from "./DynamicsCard";
import type { DynamicsRowDef, DynamicsToolProps } from "./types";

const STEREO_WIDENER_ROWS: readonly DynamicsRowDef[] = [
  {
    id: "delay",
    labelKey: "player.tools.widener.delay",
    min: 0,
    max: 75,
    step: 0.5,
    initial: 20,
    format: (value) => `${value.toFixed(1)} ms`,
  },
  {
    id: "feedback",
    labelKey: "player.tools.widener.feedback",
    min: 0,
    max: 0.9,
    step: 0.01,
    initial: 0.2,
    ticks: 8,
    format: (value) => `${value.toFixed(1)} %`,
  },
  {
    id: "crossfeed",
    labelKey: "player.tools.widener.crossfeed",
    min: 0,
    max: 0.8,
    step: 0.01,
    initial: 0.3,
    ticks: 7,
    format: (value) => `${value.toFixed(1)} %`,
  },
  {
    id: "dry-mix",
    labelKey: "player.tools.widener.dry-mix",
    min: 0,
    max: 1,
    step: 0.01,
    initial: 0.8,
    ticks: 9,
    format: (value) => `${value.toFixed(1)} %`,
  },
];

/** The Stereo Widener tool card — mountable as-is (tools dialog, effects area). */
export function StereoWidenerTool(props: DynamicsToolProps): ReactElement {
  return (
    <DynamicsCard
      tool="stereo-widener"
      nameKey="player.tools.dynamics.stereo-widener"
      pendingCommand="audio.set-stereo-widener"
      rows={STEREO_WIDENER_ROWS}
      {...props}
    />
  );
}
