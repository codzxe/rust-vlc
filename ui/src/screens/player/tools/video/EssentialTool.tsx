// w26.c4 — EssentialTool (frame: "Video - Essential.svg", 400×821): the
// 8-row picture card — collapsed 360×48 rx6 #090909 / expanded 360×666 rx6
// #1B1B1B, 59px header, rows pitch 76, **label-only** (no value text, no
// chips — the slider IS the display; the frame carries no value column):
//
//   Hue                     thumb ≈50%  · 1 degree tick
//   Brightness              thumb ≈50%  · 1 tick
//   Contrast                thumb ≈50%  · 1 tick
//   Saturation              thumb ≈33%  · 2 ticks
//   Gamma                   thumb ≈10%  · 9 ticks
//   Sharpen Sigma           thumb ≈1.5% · 3 ticks
//   Banding Removal Radius  thumb ≈44%  · 6 ticks
//   Film Grain Variance     thumb ≈20%  · 9 ticks
//
// Staged display ranges: each initial is calibrated through the shared
// slider's flush travel (w26.i07: thumb right == fill right — a value v
// paints its thumb center at ≈ 0.988·v − 0.6 % of the track) so the PAINTED
// thumb lands on the frame's SVG-measured mark ("Video - Essential.svg"
// thumb centers 50/50/50/33.5/10.4/1.8/43.0/20.4%; w26.i08 — the centered
// rows keep their neutral values: the flush grammar's −1.2pp paint shift at
// 50% is the shared Lead-sanctioned travel every tool slider uses).
// Engine ranges replace these on wiring (types.ts note).
// Hue/Brightness/Contrast/Saturation/Gamma labels REUSE the central
// customize.field.* keys.
//
// Honest staging: no adjust/sharpen/gradfun/grain engine stage or catalog
// command exists — local state + the visible staged markers (VideoCard),
// controls fully interactive.

import { useState } from "react";
import type { ReactElement } from "react";
import { VideoCard } from "./VideoCard";
import { VideoSliderRow } from "./VideoSliderRow";
import type {
  VideoSliderRowDef,
  VideoSliderRowState,
  VideoToolProps,
} from "./types";

const ESSENTIAL_ROWS: readonly VideoSliderRowDef[] = [
  {
    id: "hue",
    labelKey: "customize.field.hue", // central key — reused
    min: -180,
    max: 180,
    step: 1,
    initial: 0, // 50% — the frame's neutral-center thumb
    ticks: 1,
    format: (value) => `${Math.round(value)}°`,
  },
  {
    id: "brightness",
    labelKey: "customize.field.brightness",
    min: 0,
    max: 2,
    step: 0.01,
    initial: 1, // 50%
    ticks: 1,
    format: (value) => value.toFixed(2),
  },
  {
    id: "contrast",
    labelKey: "customize.field.contrast",
    min: 0,
    max: 2,
    step: 0.01,
    initial: 1, // 50%
    ticks: 1,
    format: (value) => value.toFixed(2),
  },
  {
    id: "saturation",
    labelKey: "customize.field.saturation",
    min: 0,
    max: 3,
    step: 0.01,
    initial: 1.04, // paints 33.6% — frame thumb center 33.5% (w26.i08)
    ticks: 2,
    format: (value) => value.toFixed(2),
  },
  {
    id: "gamma",
    labelKey: "customize.field.gamma",
    min: 0,
    max: 10,
    step: 0.1,
    initial: 1.1, // paints 10.25% — frame thumb center 10.4% (w26.i08, calibrated through the flush travel)
    ticks: 9,
    format: (value) => value.toFixed(1),
  },
  {
    id: "sharpen-sigma",
    labelKey: "player.tools.essential.sharpen-sigma",
    min: 0,
    max: 2,
    step: 0.01,
    initial: 0.05, // paints 1.86% — frame thumb center 1.8% (w26.i08)
    ticks: 3,
    format: (value) => value.toFixed(2),
  },
  {
    id: "banding-radius",
    labelKey: "player.tools.essential.banding-radius",
    min: 0,
    max: 16,
    step: 1,
    initial: 7, // paints 42.6% ≈ frame thumb center 43.0%
    ticks: 6,
    format: (value) => `${Math.round(value)} px`,
  },
  {
    id: "grain-variance",
    labelKey: "player.tools.essential.grain-variance",
    min: 0,
    max: 100,
    step: 1,
    initial: 21, // paints 20.1% ≈ frame thumb center 20.4% (w26.i08)
    ticks: 9,
    format: (value) => `${Math.round(value)} %`,
  },
];

/** The Essential tool card — mountable as-is (tools dialog, effects area). */
export function EssentialTool(props: VideoToolProps): ReactElement {
  const [values, setValues] = useState<VideoSliderRowState>(() =>
    Object.fromEntries(ESSENTIAL_ROWS.map((row) => [row.id, row.initial])),
  );

  return (
    <VideoCard
      tool="essential"
      nameKey="customize.panel.essential"
      pendingCommand="video.set-essential"
      {...props}
    >
      {ESSENTIAL_ROWS.map((row) => (
        <VideoSliderRow
          key={row.id}
          row={row}
          value={values[row.id] ?? row.initial}
          onChange={(next) => {
            setValues((prev) => ({ ...prev, [row.id]: next }));
          }}
        />
      ))}
    </VideoCard>
  );
}
