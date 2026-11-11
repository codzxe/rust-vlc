// w26.c4 — HdrTool (frame: "HDR Options.svg", 406×422): a bare dropdown —
// the ONLY tool frame without a card. Trigger 362×48 rx6 (label "Auto",
// #8F8F8F ≈15px, chevron-down); open list 362×278 rx6 #191919 with 6 plain
// rows (44px @ 46px pitch — the shared components/menu Dropdown grammar,
// a19 A9), row 1 "Auto" selected (#1F1F1F + orange check 14×11 sw2), NO
// leading icons:
//
//   Auto (selected) · Off · HDR10 · HDR10+ · Dolby Vision · HLG
//
// Frame-verified deviation from the task-card parenthetical: the frame
// carries no tone-mapping rows — just the mode dropdown ("tone mapping
// rows, dropdowns if framed": only the dropdown is framed).
//
// Honest staging: no HDR/tone-mapping engine stage or catalog command
// exists — local state + the visible staged markers, the control fully
// interactive (the shared Dropdown: roving listbox, arrows/Home/End,
// Enter/Space, Escape/Tab, typeahead).

import { useId, useState } from "react";
import type { ReactElement } from "react";
import { Dropdown } from "../../../../components/menu";
import type { DropdownOption } from "../../../../components/menu";
import { useVideoMsg } from "./copy";
import type { HdrToolProps, VideoLabelKey } from "./types";
import "./video.css";

/** The frame's six modes, in frame order (vision-confirmed labels). Mutably
 * typed: the Dropdown primitive's options prop. */
const HDR_MODES: DropdownOption<VideoLabelKey>[] = [
  { value: "auto", labelKey: "player.tools.hdr.auto" },
  { value: "off", labelKey: "player.tools.hdr.off" },
  { value: "hdr10", labelKey: "player.tools.hdr.hdr10" },
  { value: "hdr10-plus", labelKey: "player.tools.hdr.hdr10-plus" },
  { value: "dolby-vision", labelKey: "player.tools.hdr.dolby-vision" },
  { value: "hlg", labelKey: "player.tools.hdr.hlg" },
];

/** The HDR options section — mountable as-is (tools dialog, effects area).
 * Frame-exact at a 362px host (the 406px frame's 22px insets); fluid below. */
export function HdrTool({ id: idProp }: HdrToolProps): ReactElement {
  const m = useVideoMsg();
  const [mode, setMode] = useState("auto"); // frame: "Auto" selected
  const generatedId = useId();
  const id = idProp ?? `hdr-${generatedId}`;

  return (
    <section
      className="video-hdr"
      id={id}
      data-tool="hdr"
      data-staged-tool="hdr"
      data-pending-command="video.set-hdr"
      aria-label={m("customize.panel.hdr")}
    >
      <Dropdown
        options={HDR_MODES}
        value={mode}
        onChange={setMode}
        label={m("player.tools.hdr.mode")}
        resolveLabel={m}
      />
      <p className="video-hdr__staged" role="note">
        {m("player.tools.video.staged")}
      </p>
    </section>
  );
}
