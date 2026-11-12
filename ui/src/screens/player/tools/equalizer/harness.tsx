// w26.c1 — DEV-ONLY vision harness (never imported by app routes; the
// w26.b10 playlist-options harness pattern): composes the EQ surfaces'
// frame states on the app surface so the comparison loop (contract §5) can
// screenshot them next to design/figma-renders/{EQ,EQ_presets}.png.
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly at the A2 frame scale (824px canvas — the
// renders' 900px width at ~1.09 device scale) — see the lane log for the
// loop record.
//
//   W26C1_HARNESS_OUT=/tmp/w26c1-harness npm run test -- harness.test

import type { ReactElement } from "react";
import { EqualizerTool } from "./EqualizerTool";
import "./harness.css";

/**
 * States 1+2 — the EQ.svg frame's two stacked panel states: the collapsed
 * 784×36 "Open Equalizer" bar at the top, the expanded "Close Equalizer"
 * panel below it (frame offsets: 20px canvas inset, 20px between the
 * states).
 */
export function EqualizerCardStates(): ReactElement {
  return (
    <div className="w26c1-stage">
      <EqualizerTool />
      <EqualizerTool defaultOpen />
    </div>
  );
}

/**
 * State 3 — the expanded tool with the preset list open (EQ presets.svg
 * state=open). The trigger is click-gated, so the harness test opens the
 * list through the public Dropdown API before serializing.
 */
export function EqualizerPresetsHarness(): ReactElement {
  return (
    <div className="w26c1-stage">
      <EqualizerTool defaultOpen />
    </div>
  );
}
