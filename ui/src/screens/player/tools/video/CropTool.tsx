// w26.c4 — CropTool (frame: "Video - Crop.svg", 400×821): the smallest tool
// card — collapsed 360×48 rx6 #090909 / expanded 360×152 rx6 #1B1B1B, 59px
// header, then two rows of paired number inputs at a 49px pitch:
//
//   [Top 0 px] [Bottom 0 px] (link)
//   [Left 0 px] [Right 0 px] (link)
//
// Input anatomy: label inside-left (#6B6B6B ≈15px @ 12.7px inset), value "0"
// (#8F8F8F, tabular) + unit "px" (#404040), the value+unit group ending 41px
// from the input's right edge. The link icon (the frame's "Rectangle 14",
// 20×10, sw1.5 #8F8F8F, 21px from the card's right edge) is a row-level
// toggle: chain = pair locked (Top↔Bottom, Left↔Right — editing one mirrors
// the other).
//
// Frame-verified deviation from the task-card parenthetical: the frame
// carries NO aspect-ratio presets — only the four pixel inputs + link
// toggles (the frame is the design authority; the presets live in the
// customize panel / Settings→Video framing rows, not here).
//
// Honest staging: no crop engine stage or catalog command exists — local
// state + the visible staged markers (VideoCard), controls fully interactive.

import { useId, useState } from "react";
import type { ReactElement } from "react";
import { VideoCard } from "./VideoCard";
import { useVideoMsg } from "./copy";
import type { CropAxis, VideoToolProps } from "./types";

/** Digit-only sanitize (crop pixels are non-negative integers). */
const digits = (value: string): string => value.replace(/[^\d]/g, "");

/** The four frame fields, in frame order. */
interface CropFields {
  top: string;
  bottom: string;
  left: string;
  right: string;
}

const FRAME_CROP: CropFields = { top: "0", bottom: "0", left: "0", right: "0" };

type CropField = keyof CropFields;

/** The frame's two coupled pairs, keyed by crop axis — the link icon locks a
 * pair (Top↔Bottom, Left↔Right). w26.k11 (j11 P3-3): module-level (the
 * former per-keystroke closure rebuild is gone) and typed through the
 * exported CropAxis, which was dead before this map consumed it. */
const CROP_PAIRS: Readonly<Record<CropAxis, readonly [CropField, CropField]>> = {
  vertical: ["top", "bottom"],
  horizontal: ["left", "right"],
};

/** The axis a field's link toggle governs. */
const axisOf = (field: CropField): CropAxis =>
  field === "top" || field === "bottom" ? "vertical" : "horizontal";

/** One 141×33 number input: inner label left, value+unit right. */
function CropInput(props: {
  label: string;
  unit: string;
  value: string;
  onChange: (next: string) => void;
  inputId: string;
}): ReactElement {
  return (
    <span className="video-crop-input">
      <label className="video-crop-input__label" htmlFor={props.inputId}>
        {props.label}
      </label>
      <span className="video-crop-input__value">
        <input
          id={props.inputId}
          className="video-crop-input__control"
          type="text"
          inputMode="numeric"
          value={props.value}
          onChange={(event) => {
            props.onChange(digits(event.target.value));
          }}
        />
        <span className="video-crop-input__unit" aria-hidden="true">
          {props.unit}
        </span>
      </span>
    </span>
  );
}

/** One pair row: two inputs + the link toggle that couples them. */
function CropPairRow(props: {
  first: { label: string; value: string };
  second: { label: string; value: string };
  unit: string;
  linkLabel: string;
  linked: boolean;
  onToggleLink: () => void;
  onFirst: (next: string) => void;
  onSecond: (next: string) => void;
  rowId: string;
}): ReactElement {
  return (
    <div className="video-crop-row" data-row={props.rowId}>
      <CropInput
        label={props.first.label}
        unit={props.unit}
        value={props.first.value}
        onChange={props.onFirst}
        inputId={`${props.rowId}-first`}
      />
      <CropInput
        label={props.second.label}
        unit={props.unit}
        value={props.second.value}
        onChange={props.onSecond}
        inputId={`${props.rowId}-second`}
      />
      <button
        type="button"
        className="video-crop-link"
        aria-pressed={props.linked}
        aria-label={props.linkLabel}
        onClick={props.onToggleLink}
      >
        {/* frame: Video - Crop.svg — "Rectangle 14" verbatim (two
         * interlocking links, 20×10, stroke 1.5 #8F8F8F → currentColor,
         * which carries the frame ink through --color-content-secondary). */}
        <svg
          viewBox="339 153.5 20 10"
          width="20"
          height="10"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M351.5 153.5H354C356.761 153.5 359 155.739 359 158.5C359 161.261 356.761 163.5 354 163.5H351.5M346.5 163.5H344C341.239 163.5 339 161.261 339 158.5C339 155.739 341.239 153.5 344 153.5H346.5" />
        </svg>
      </button>
    </div>
  );
}

/** The Crop tool card — mountable as-is (tools dialog, effects area). */
export function CropTool({
  defaultOpen = false,
  id: idProp,
}: VideoToolProps): ReactElement {
  const m = useVideoMsg();
  const [crop, setCrop] = useState<CropFields>(FRAME_CROP);
  const [verticalLinked, setVerticalLinked] = useState(true);
  const [horizontalLinked, setHorizontalLinked] = useState(true);
  const generatedId = useId();
  const id = idProp ?? `crop-${generatedId}`;

  /** Setting one field of a pair mirrors it when the pair is linked. */
  const setField = (field: CropField, next: string): void => {
    setCrop((prev) => {
      const [first, second] = CROP_PAIRS[axisOf(field)];
      const linked =
        axisOf(field) === "vertical" ? verticalLinked : horizontalLinked;
      if (!linked) return { ...prev, [field]: next };
      const twin = field === first ? second : first;
      return { ...prev, [field]: next, [twin]: next };
    });
  };

  return (
    <VideoCard
      tool="crop"
      nameKey="customize.field.crop"
      pendingCommand="video.set-crop"
      defaultOpen={defaultOpen}
      id={id}
    >
      <CropPairRow
        rowId={`${id}-vertical`}
        first={{ label: m("player.tools.crop.top"), value: crop.top }}
        second={{ label: m("player.tools.crop.bottom"), value: crop.bottom }}
        unit={m("player.tools.crop.unit")}
        linkLabel={m("player.tools.crop.link-vertical")}
        linked={verticalLinked}
        onToggleLink={() => {
          setVerticalLinked((prev) => !prev);
        }}
        onFirst={(next) => {
          setField("top", next);
        }}
        onSecond={(next) => {
          setField("bottom", next);
        }}
      />
      <CropPairRow
        rowId={`${id}-horizontal`}
        first={{ label: m("player.tools.crop.left"), value: crop.left }}
        second={{ label: m("player.tools.crop.right"), value: crop.right }}
        unit={m("player.tools.crop.unit")}
        linkLabel={m("player.tools.crop.link-horizontal")}
        linked={horizontalLinked}
        onToggleLink={() => {
          setHorizontalLinked((prev) => !prev);
        }}
        onFirst={(next) => {
          setField("left", next);
        }}
        onSecond={(next) => {
          setField("right", next);
        }}
      />
    </VideoCard>
  );
}
