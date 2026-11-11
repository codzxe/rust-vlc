// w26.c3 — DEV-ONLY vision harness (never imported by app routes; the
// w26.c2/c1 harness pattern): composes the visualizer selector's two frame
// states + the tool card's product states so the comparison loop
// (contract §5) can screenshot them against
// design/figma-renders/Audio_Visualizer.png.
//
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly at the frame scale (406px canvas × 2.2167 =
// the render's 900px width) — see the lane log for the loop record.
//
//   W26C3_HARNESS_OUT=/tmp/w26c3-harness npm run test -- harness.test

import type { ReactElement } from "react";
import { VisualizerModeSelect } from "./VisualizerModeSelect";
import { VisualizerTool } from "./VisualizerTool";
import { useVisualizerMsg } from "./useVisualizerMsg";
import "./harness.css";

/** One labeled cell (the c2 pattern — captions are prop pass-throughs, not
 * JSX copy; omitted on the frame-exact cell so its shot composes 1:1
 * against the render). */
function HarnessCell(props: {
  caption?: string;
  children: ReactElement[];
}): ReactElement {
  return (
    <div className="w26c3-cell">
      {props.caption !== undefined && (
        <p className="w26c3-caption">{props.caption}</p>
      )}
      {props.children}
    </div>
  );
}

/**
 * The frame cell — "Audio Visualizer.svg"'s two stacked states: the closed
 * trigger @ y22 and the open dropdown @ y90 (20px apart, 22px gutters). The
 * SECOND selector is the one harness.test opens before serializing; the
 * list drops below its trigger (the a19 Dropdown anchoring — the frames
 * document the two states separately; see the lane log).
 */
export function VisualizerSelectorStates(): ReactElement {
  const m = useVisualizerMsg();
  return (
    <div className="w26c3-stage">
      {/* No caption — this cell mirrors the frame canvas exactly (closed
       * trigger at y22, open composition at y90). */}
      <HarnessCell>
        <VisualizerModeSelect
          value="disabled"
          onChange={() => {}}
          label={m("player.tools.visualizer.mode")}
          resolveLabel={m}
        />
        <div className="w26c3-frame-open">
          <VisualizerModeSelect
            value="disabled"
            onChange={() => {}}
            label={m("player.tools.visualizer.mode")}
            resolveLabel={m}
          />
        </div>
      </HarnessCell>
    </div>
  );
}

/**
 * The product cell — the mountable tool card: collapsed (frame default),
 * expanded-Disabled (the frame's own resting state: selector only), and
 * two expanded examples with the display (spectrum with live levels via
 * the eq token group; waveform at rest — the frame's midline grammar).
 */
export function VisualizerToolStates(): ReactElement {
  return (
    <div className="w26c3-stage">
      <HarnessCell caption="Visualizer tool card">
        <VisualizerTool />
        <VisualizerTool defaultOpen />
        <VisualizerTool
          defaultOpen
          defaultMode="spectrum"
          levels={[0.2, 0.55, 0.35, 0.9, 0.5, 0.72, 0.4, 0.65, 0.3, 0.84, 0.45, 0.6]}
        />
        <VisualizerTool defaultOpen defaultMode="waveform" />
      </HarnessCell>
    </div>
  );
}
