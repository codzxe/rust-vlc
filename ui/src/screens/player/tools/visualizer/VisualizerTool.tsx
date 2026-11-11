// w26.c3 — VisualizerTool: the audio visualizer tool card, composed to
// design/figma/"Audio Visualizer.svg" (the mode dropdown — w26.a20 §A7)
// inside the shared tool-card grammar (§D1, the w26.c1/w26.c2 siblings):
//
//   * collapsible card — collapsed 360×48 rx6 #090909 "Expand Settings" +
//     chevron-›; expanded rx6 #1B1B1B card whose 59px header carries
//     "Collapse Settings" + chevron-∨ (one morphing surface);
//   * body — the levels-driven VisualizerDisplay (honest: no FFT source —
//     absent levels render the mode's RESTING frame depiction, never
//     simulated data) above the VisualizerModeSelect (the frame's five-mode
//     dropdown with its two-tone icons);
//   * HONEST STAGING (the effects-area pattern, settings/areas/effects.tsx):
//     no visualizer engine stage or catalog command exists — the selector
//     stays fully interactive with LOCAL state, visibly marked staged
//     (role="note" + data-pending-command naming the forward-declared
//     command it awaits), never a dead control that looks engine-bound.
//
// Mountable as-is (tools dialog via ToolsDialogShell, the Settings → Audio
// effects area, a16's music mode): all state is local; `onChange` is the
// wire seam; `levels` is the host-owned data seam (readonly — push a new
// array per animation frame when the analysis source lands).

import { useId, useState } from "react";
import type { ReactElement } from "react";
import { Icon } from "../../../../components/icon/Icon";
import { VisualizerDisplay } from "./VisualizerDisplay";
import { VisualizerModeSelect } from "./VisualizerModeSelect";
import { useVisualizerMsg } from "./useVisualizerMsg";
import type { VisualizerMode, VisualizerToolProps } from "./types";
import "./visualizer.css";

/** Forward-declared command the mode selection awaits (staged marker). */
const PENDING_COMMAND = "audio.set-visualizer";

export function VisualizerTool({
  defaultOpen = false,
  defaultMode = "disabled",
  levels,
  onChange,
  id: idProp,
  className = "",
}: VisualizerToolProps): ReactElement {
  const m = useVisualizerMsg();
  const [open, setOpen] = useState(defaultOpen);
  const [mode, setMode] = useState<VisualizerMode>(defaultMode);
  const generatedId = useId();
  const id = idProp ?? `visualizer-${generatedId}`;
  const bodyId = `${id}-body`;

  const commitMode = (next: VisualizerMode): void => {
    setMode(next);
    onChange?.(next);
  };

  return (
    <section
      className={["viz-tool", open ? "viz-tool--open" : "", className]
        .filter(Boolean)
        .join(" ")}
      data-tool="visualizer"
      data-staged-tool="visualizer"
      data-pending-command={PENDING_COMMAND}
      aria-label={m("player.tools.visualizer.title")}
    >
      <button
        type="button"
        id={id}
        className="viz-tool__toggle"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => {
          setOpen((prev) => !prev);
        }}
      >
        <span className="viz-tool__toggle-label">
          {m(open ? "player.tools.collapse-settings" : "player.tools.expand-settings")}
        </span>
        {/* Collapsed: chevron-› (frame Vector 27); expanded rotates to ∨ —
         * the shared glyph, CSS rotates it (the c1/c2 card pattern). */}
        <Icon name="chevron-right" size={24} className="viz-tool__chevron" />
      </button>
      <div
        id={bodyId}
        className="viz-tool__body"
        role="group"
        aria-labelledby={id}
        hidden={!open}
      >
        {/* The visualization area — renders nothing while Disabled (the
         * frame's default state is the dropdown alone). */}
        <VisualizerDisplay
          mode={mode}
          levels={levels}
          resolveLabel={m}
          className="viz-tool__display"
        />
        {/* The frame's mode dropdown ("Audio Visualizer.svg"). */}
        <VisualizerModeSelect
          value={mode}
          onChange={commitMode}
          label={m("player.tools.visualizer.mode")}
          resolveLabel={m}
          className="viz-tool__mode"
        />
        <p className="viz-tool__staged" role="note">
          {m("player.tools.visualizer.staged")}
        </p>
      </div>
    </section>
  );
}
