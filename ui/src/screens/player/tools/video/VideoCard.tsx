// w26.c4 — VideoCard: the video tools' shared collapsible card, per the
// tool-card grammar (w26.a20 analysis §B1–B3/§D1; frames "Video - Colors.svg",
// "Video - Crop.svg", "Video - Essential.svg" — identical shells, only the
// row sets differ):
//
//   COLLAPSED  360×48 rx6 #090909 card · "Expand Settings" #6B6B6B ≈14px ·
//              chevron-› 5×10 (Vector 27 family).
//   EXPANDED   360×H rx6 #1B1B1B card · 59px header "Collapse Settings" +
//              chevron-∨ INSIDE the card (one surface, not bar+card) ·
//              20px bottom pad.
//
// COMPOSITION, NOT DUPLICATION (mission pin): the shell IS the w26.c2
// dynamics card grammar, so this component reuses the dynamics-tool classes
// and imports the dynamics stylesheet verbatim — collapsed pill, expanded
// panel, 59px header, chevron states and staged-note look come from
// ../dynamics/dynamics.css. What differs from DynamicsCard is the body: the
// video frames carry heterogeneous rows (hex+swatch+toggle lines, crop
// input pairs, a sub-dropdown), which the dynamics card's sliders-only
// `rows` API cannot host, so VideoCard takes free `children` (and its
// staged note names the VIDEO effects command, not the audio one). The
// dynamics module itself is never edited.
//
// Chevron direction encodes the state (› collapsed / ∨ expanded — analysis
// §D1 pitfall 7); the expanded body stays in the DOM (`hidden`) so
// aria-controls always resolves.
//
// HONEST STAGING (the effects-area pattern, settings/areas/effects.tsx): no
// video-effects engine stage or catalog command exists. Controls stay fully
// interactive with LOCAL state, visibly marked staged — a role="note" line +
// data-pending-command naming the forward-declared command they await.

import { useId, useState } from "react";
import type { ReactElement } from "react";
import { Icon } from "../../../../components/icon/Icon";
import { useVideoMsg } from "./copy";
import type { VideoCardProps } from "./types";
// The shared tool-card grammar (collapsed pill / expanded panel / rows /
// staged note) — pulled in here so every mounting host gets it once.
import "../dynamics/dynamics.css";
import "./video.css";

/** Renders a video tool card (see module header for the grammar). */
export function VideoCard({
  tool,
  nameKey,
  pendingCommand,
  children,
  defaultOpen = false,
  id: idProp,
}: VideoCardProps): ReactElement {
  const m = useVideoMsg();
  const [open, setOpen] = useState(defaultOpen);
  const generatedId = useId();
  const id = idProp ?? `${tool}-${generatedId}`;
  const bodyId = `${id}-body`;

  return (
    <section
      className={[
        "dynamics-tool",
        "video-tool",
        `video-tool--${tool}`,
        open ? "dynamics-tool--open" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      data-tool={tool}
      data-staged-tool={tool}
      data-pending-command={pendingCommand}
      aria-label={m(nameKey)}
    >
      <button
        type="button"
        id={id}
        className="dynamics-tool__toggle"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => {
          setOpen((prev) => !prev);
        }}
      >
        <span className="dynamics-tool__toggle-label">
          {open
            ? m("player.tools.collapse-settings")
            : m("player.tools.expand-settings")}
        </span>
        <Icon
          name={open ? "chevron-down" : "chevron-right"}
          className="dynamics-tool__chevron"
        />
      </button>
      <div
        id={bodyId}
        className="dynamics-tool__body"
        role="group"
        aria-labelledby={id}
        hidden={!open}
      >
        <div className="dynamics-tool__rows">{children}</div>
        <p className="dynamics-tool__staged" role="note">
          {m("player.tools.video.staged")}
        </p>
      </div>
    </section>
  );
}
