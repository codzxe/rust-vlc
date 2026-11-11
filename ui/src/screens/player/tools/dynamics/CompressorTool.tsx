// w26.c2 — CompressorTool (frame: "Audio - Compressor Settings.svg",
// 400×821): the 7-row compressor card — collapsed 360×48 rx6 #090909 /
// expanded 360×590 rx6 #1B1B1B, header 59px, rows pitch 76, PLAIN values
// (no chips), sliders with NO degree ticks in this frame.
//
// Defaults are the frame's value column (VLC compressor semantics: RMS/peak
// 0.2, attack 25 ms, release 100 ms, threshold −11 dB, ratio 4:1, knee
// 5 dB, makeup 7 dB). Ranges are staged display ranges calibrated to the
// frame's measured thumb positions (types.ts note); the first label is the
// design's "FM5 / Peak" typo, corrected to "RMS / Peak" (w26.a20 §A4/D4).

import type { ReactElement } from "react";
import { DynamicsCard } from "./DynamicsCard";
import type { DynamicsRowDef, DynamicsToolProps } from "./types";

const COMPRESSOR_ROWS: readonly DynamicsRowDef[] = [
  {
    id: "rms-peak",
    labelKey: "player.tools.compressor.rms-peak",
    min: 0,
    max: 1.25,
    step: 0.05,
    initial: 0.2,
    format: (value) => value.toFixed(1),
  },
  {
    id: "attack",
    labelKey: "player.tools.compressor.attack",
    min: 0,
    max: 280,
    step: 0.5,
    initial: 25,
    format: (value) => `${value.toFixed(1)} ms`,
  },
  {
    id: "release",
    labelKey: "player.tools.compressor.release",
    min: 0,
    max: 600,
    step: 1,
    initial: 100,
    format: (value) => `${value.toFixed(1)} ms`,
  },
  {
    id: "threshold",
    labelKey: "player.tools.compressor.threshold",
    min: -30,
    max: 0,
    step: 0.5,
    initial: -11,
    format: (value) => `${value.toFixed(1)} dB`,
  },
  {
    id: "ratio",
    labelKey: "player.tools.compressor.ratio",
    min: 1,
    max: 10,
    step: 0.5,
    initial: 4,
    format: (value) => `${value.toFixed(1)} : 1`,
  },
  {
    id: "knee",
    labelKey: "player.tools.compressor.knee",
    min: 0,
    max: 10,
    step: 0.5,
    initial: 5,
    format: (value) => `${value.toFixed(1)} dB`,
  },
  {
    id: "makeup",
    labelKey: "player.tools.compressor.makeup",
    min: 0,
    max: 21,
    step: 0.5,
    initial: 7,
    format: (value) => `${value.toFixed(1)} dB`,
  },
];

/** The Compressor tool card — mountable as-is (tools dialog, effects area). */
export function CompressorTool(props: DynamicsToolProps): ReactElement {
  return (
    <DynamicsCard
      tool="compressor"
      nameKey="player.tools.dynamics.compressor"
      pendingCommand="audio.set-compressor"
      rows={COMPRESSOR_ROWS}
      {...props}
    />
  );
}
