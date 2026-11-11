// w26.c1 — DEV-ONLY vision harness (never imported by app routes; the
// w26.b10 playlist-options harness pattern): composes the EQ tool card's
// frame states on the app surface so the comparison loop (contract §5) can
// screenshot them next to design/figma-renders/{Audio_-_Equalizer,EQ,
// EQ_presets}.png.
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly at the A1 frame scale (400px canvas × 2.25 =
// the renders' 900px width) — see the lane log for the loop record.
//
//   W26C1_HARNESS_OUT=/tmp/w26c1-harness npm run test -- harness.test

import type { ReactElement } from "react";
import { EqualizerTool } from "./EqualizerTool";
import "./harness.css";

/**
 * State 1+2 — the Audio - Equalizer.svg frame's two stacked card states:
 * collapsed 360×48 card at the top, the expanded card below it (frame
 * offsets: 20px canvas inset, 20px between the states).
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
 * State 3 — the expanded card with the preset list open (EQ presets.svg
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
