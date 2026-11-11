// w26.c2 — SpatializerTool (frame: "Audio - Spatializer Settings.svg",
// 400×821): the 5-row spatializer card — collapsed 360×48 rx6 #090909 /
// expanded 360×438 rx6 #1B1B1B, header 59px, rows pitch 76, PLAIN values,
// and degree ticks on EVERY row (the only all-ticked dynamics frame).
//
// Tick counts are the frame's data-driven degree marks: Size 10, the other
// rows 9 (analysis §A5) — with each default sitting exactly on a tick
// (Size 8 of 11 steps, Width at the max end, Wet/Dry/Damp on the 1..9 marks
// of a 0..10 scale). Ranges are staged display ranges (types.ts note).

import type { ReactElement } from "react";
import { DynamicsCard } from "./DynamicsCard";
import type { DynamicsRowDef, DynamicsToolProps } from "./types";

const SPATIALIZER_ROWS: readonly DynamicsRowDef[] = [
  {
    id: "size",
    labelKey: "player.tools.spatializer.size",
    min: 0,
    max: 11,
    step: 0.5,
    initial: 8,
    ticks: 10,
    format: (value) => value.toFixed(1),
  },
  {
    id: "width",
    labelKey: "player.tools.spatializer.width",
    min: 0,
    max: 10,
    step: 0.5,
    initial: 10,
    ticks: 9,
    format: (value) => value.toFixed(1),
  },
  {
    id: "wet",
    labelKey: "player.tools.spatializer.wet",
    min: 0,
    max: 10,
    step: 0.5,
    initial: 4,
    ticks: 9,
    format: (value) => value.toFixed(1),
  },
  {
    id: "dry",
    labelKey: "player.tools.spatializer.dry",
    min: 0,
    max: 10,
    step: 0.5,
    initial: 5,
    ticks: 9,
    format: (value) => value.toFixed(1),
  },
  {
    id: "damp",
    labelKey: "player.tools.spatializer.damp",
    min: 0,
    max: 10,
    step: 0.5,
    initial: 5,
    ticks: 9,
    format: (value) => value.toFixed(1),
  },
];

/** The Spatializer tool card — mountable as-is (tools dialog, effects area). */
export function SpatializerTool(props: DynamicsToolProps): ReactElement {
  return (
    <DynamicsCard
      tool="spatializer"
      nameKey="player.tools.dynamics.spatializer"
      pendingCommand="audio.set-spatializer"
      rows={SPATIALIZER_ROWS}
      {...props}
    />
  );
}
