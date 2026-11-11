// w26.c2 — DEV-ONLY vision harness (never imported by app routes): composes
// the three dynamics tools, each as the frames present them — the collapsed
// card above the expanded card, in a 400px frame cell — so the comparison
// loop (contract §5) can screenshot them against
// design/figma-renders/Audio_-_Compressor_Settings.png,
// Audio_-_Spatializer_Settings.png and Audio_-_Stereo_Widener.png.
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly — see the lane log for the loop record.
//
//   W26C2_HARNESS_OUT=/tmp/w26c2-harness npm run test -- harness.test

import type { ReactElement } from "react";
import { CompressorTool } from "./CompressorTool";
import { SpatializerTool } from "./SpatializerTool";
import { StereoWidenerTool } from "./StereoWidenerTool";
import "./harness.css";

/** One frame cell: collapsed card over expanded card (the frames' two
 * stacked Property states), 20px gutters — the cards land at 360px wide. */
function HarnessCell(props: {
  caption: string;
  children: ReactElement[];
}): ReactElement {
  return (
    <div className="w26c2-cell">
      {/* Dev-only harness caption: the figma frame name itself, not product
       * copy (plain prop pass-through — the lint rule doesn't fire here). */}
      <p className="w26c2-caption">{props.caption}</p>
      <div className="w26c2-frame">{props.children}</div>
    </div>
  );
}

/** The three dynamics tools side by side, frame-layout each. */
export function DynamicsToolsHarness(): ReactElement {
  return (
    <div className="w26c2-stage">
      <HarnessCell caption="Audio - Compressor Settings">
        <CompressorTool />
        <CompressorTool defaultOpen />
      </HarnessCell>
      <HarnessCell caption="Audio - Spatializer Settings">
        <SpatializerTool />
        <SpatializerTool defaultOpen />
      </HarnessCell>
      <HarnessCell caption="Audio - Stereo Widener">
        <StereoWidenerTool />
        <StereoWidenerTool defaultOpen />
      </HarnessCell>
    </div>
  );
}
