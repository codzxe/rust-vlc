// w26.c4 — DEV-ONLY vision harness (never imported by app routes): composes
// the four video tools, each as its frame presents the states — the collapsed
// card above the expanded card, in a frame-sized canvas — so the comparison
// loop (contract §5) can screenshot them 1:1 (2.25 device scale = the 900px
// renders) against design/figma-renders/Video_-_Colors.png, Video_-_Crop.png,
// Video_-_Essential.png and HDR_Options.png.
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly — see the lane log for the loop record.
//
//   W26C4_HARNESS_OUT=/tmp/w26c4-harness npm run test -- harness.test

import type { ReactElement } from "react";
import { ColorsTool } from "./ColorsTool";
import { CropTool } from "./CropTool";
import { EssentialTool } from "./EssentialTool";
import { HdrTool } from "./HdrTool";
import "./harness.css";

/** One frame canvas: collapsed card over expanded card (the frames' two
 * stacked Property states), 20px state gap — cards land at 360px wide. */
function CardFrameStates(props: {
  caption: string;
  children: ReactElement[];
}): ReactElement {
  return (
    <div className="w26c4-stage">
      {/* Dev-only harness caption: the figma frame name itself, not product
       * copy (plain prop pass-through — the lint rule doesn't fire here). */}
      <p className="w26c4-caption">{props.caption}</p>
      <div className="w26c4-frame">{props.children}</div>
    </div>
  );
}

/** State 1+2 — Video - Colors.svg (400×888): both stacked card states. */
export function ColorsHarness(): ReactElement {
  return (
    <CardFrameStates caption="Video - Colors">
      <ColorsTool />
      <ColorsTool defaultOpen />
    </CardFrameStates>
  );
}

/** State 1+2 — Video - Crop.svg (400×821). */
export function CropHarness(): ReactElement {
  return (
    <CardFrameStates caption="Video - Crop">
      <CropTool />
      <CropTool defaultOpen />
    </CardFrameStates>
  );
}

/** State 1+2 — Video - Essential.svg (400×821). */
export function EssentialHarness(): ReactElement {
  return (
    <CardFrameStates caption="Video - Essential">
      <EssentialTool />
      <EssentialTool defaultOpen />
    </CardFrameStates>
  );
}

/** State 1+2 — HDR Options.svg (406×422): the closed trigger above a second
 * HdrTool whose list harness.test.tsx opens through the public Dropdown API
 * before serializing (the w26.c1 presets-open pattern). */
const HDR_HARNESS_CAPTION = "HDR Options";

export function HdrHarness(): ReactElement {
  return (
    <div className="w26c4-stage w26c4-stage--hdr">
      <p className="w26c4-caption">{HDR_HARNESS_CAPTION}</p>
      <div className="w26c4-frame w26c4-frame--hdr">
        <HdrTool />
        <div className="w26c4-hdr-open">
          <HdrTool />
        </div>
      </div>
    </div>
  );
}
