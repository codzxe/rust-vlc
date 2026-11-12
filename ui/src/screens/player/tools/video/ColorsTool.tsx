// w26.c4 — ColorsTool (frame: "Video - Colors.svg", 400×888): the richest
// video tool card — collapsed 360×48 rx6 #090909 / expanded 360×755 rx6
// #1B1B1B, 59px header, heterogeneous rows sectioned by 1px separators:
//
//   Color Extraction   label · hex input "#FF0000" + 22×22 swatch · toggle OFF
//   ——— separator
//   Color Threshold    same hex+swatch+toggle row
//   Saturation         label-only slider (thumb ≈20%)
//   Similarity         label-only slider (thumb ≈20%)
//   ——— separator
//   Negate Colors      label · toggle OFF
//   ——— separator
//   Posterize          label · toggle OFF
//   ——— separator
//   Gradient           label · toggle OFF
//   [Gradient mode]    sub-dropdown 328×48 ("Gradient")
//   Color              label · toggle OFF
//   Cartoon            label · toggle OFF
//   ——— separator
//   Sepia Intensity    label-only slider (thumb ≈34%, 2 degree ticks)
//
// Frame-verified deviations from the task-card parenthetical: the frame
// carries NO brightness/contrast/hue/gamma rows here (those live in the
// Essential frame) and NO value chips — its slider rows are label-only, and
// there is no color-wheel/thumbnails section anywhere in the frame
// (vision-confirmed). The frames win (the w26.c2 Decision-1 precedent).
//
// Honest staging: no color-threshold/negate/posterize/gradient/cartoon/sepia
// engine stage or catalog command exists — local state + the visible staged
// markers (VideoCard), controls fully interactive.

import { useId, useState } from "react";
import type { ReactElement } from "react";
import { Toggle } from "../../../../components/button/Toggle";
import { Dropdown } from "../../../../components/menu";
import type { DropdownOption } from "../../../../components/menu";
import { VideoCard } from "./VideoCard";
import { VideoSliderRow } from "./VideoSliderRow";
import { useVideoMsg } from "./copy";
import { useDropdownScrollportFlip } from "./useDropdownScrollportFlip";
import {
  FRAME_EXTRACTION_COLOR,
  FRAME_THRESHOLD_COLOR,
  isHexColor,
} from "./frameData";
import type { VideoLabelKey, VideoSliderRowDef, VideoToolProps } from "./types";

/** Frame value format: no visible value column — this feeds aria-valuetext. */
const percent = (value: number): string => `${Math.round(value)} %`;

/** Saturation / Similarity / Sepia Intensity — STAGED DISPLAY RANGES: each
 * initial is calibrated through the shared slider's flush travel (w26.i07:
 * thumb right == fill right, `left: calc((100% - 4px) * p / 100 - 6px)` — a
 * value v paints its thumb center at ≈ 0.988·v − 0.6 % of the track) so the
 * PAINTED thumb lands on the frame's SVG-measured mark ("Video - Colors.svg":
 * 20.73% / 20.73% / 33.54%; w26.i08 — the pre-flush initials 20/20/34 sat
 * 1.3–5.3px left of the frame after i07's formula landed). */
const SATURATION_ROW: VideoSliderRowDef = {
  id: "saturation",
  labelKey: "customize.field.saturation", // central key — reused
  min: 0,
  max: 100,
  step: 1,
  initial: 21.6, // paints 20.7% — frame thumb center 20.73%
  format: percent,
};

const SIMILARITY_ROW: VideoSliderRowDef = {
  id: "similarity",
  labelKey: "player.tools.colors.similarity",
  min: 0,
  max: 100,
  step: 1,
  initial: 21.6, // paints 20.7% — frame thumb center 20.73%
  format: percent,
};

const SEPIA_ROW: VideoSliderRowDef = {
  id: "sepia",
  labelKey: "player.tools.colors.sepia",
  min: 0,
  max: 100,
  step: 1,
  initial: 34.55, // paints 33.5% — frame thumb center 33.54%
  ticks: 2, // frame: 2 degree ticks @ x146/x254
  format: percent,
};

const COLOR_SLIDERS = [SATURATION_ROW, SIMILARITY_ROW, SEPIA_ROW] as const;

/** The Gradient mode sub-dropdown's options — VLC's gradient filter modes
 * (the frame exports only the closed trigger; option set [VERIFY engine
 * lane]). Mutably typed: the Dropdown primitive's options prop. */
const GRADIENT_MODES: DropdownOption<VideoLabelKey>[] = [
  { value: "gradient", labelKey: "player.tools.gradient.gradient" },
  { value: "edge", labelKey: "player.tools.gradient.edge" },
  { value: "hemboss", labelKey: "player.tools.gradient.hemboss" },
];

/** Toggle ids — every frame toggle ships OFF. */
type ColorsToggleId =
  | "extraction"
  | "threshold"
  | "negate"
  | "posterize"
  | "gradient"
  | "color"
  | "cartoon";

const FRAME_TOGGLES: Readonly<Record<ColorsToggleId, VideoLabelKey>> = {
  extraction: "player.tools.colors.extraction",
  threshold: "player.tools.colors.threshold",
  negate: "player.tools.colors.negate",
  posterize: "player.tools.colors.posterize",
  gradient: "player.tools.colors.gradient",
  color: "player.tools.colors.color",
  cartoon: "player.tools.colors.cartoon",
};

const ALL_OFF: Readonly<Record<ColorsToggleId, boolean>> = {
  extraction: false,
  threshold: false,
  negate: false,
  posterize: false,
  gradient: false,
  color: false,
  cartoon: false,
};

/** One hex+swatch+toggle row (Color Extraction / Color Threshold). */
function ColorsHexRow(props: {
  label: string;
  hex: string;
  onHex: (next: string) => void;
  checked: boolean;
  onToggle: () => void;
  inputId: string;
}): ReactElement {
  return (
    <div className="video-row video-row--hex" data-row="hex">
      <label className="video-row__label" htmlFor={props.inputId}>
        {props.label}
      </label>
      <span className="video-row__spacer" aria-hidden="true" />
      <span className="video-hex">
        <input
          id={props.inputId}
          className="video-hex__input"
          type="text"
          value={props.hex}
          spellCheck={false}
          maxLength={7}
          onChange={(event) => {
            props.onHex(event.target.value);
          }}
        />
        {/* The swatch fill is the field's DATA (the editable color), not a
         * themed surface — dynamic inline style by design. */}
        <span
          className="video-hex__swatch"
          style={
            isHexColor(props.hex)
              ? { backgroundColor: props.hex }
              : undefined
          }
          aria-hidden="true"
        />
      </span>
      <Toggle checked={props.checked} onChange={props.onToggle} aria-label={props.label} />
    </div>
  );
}

/** One label+toggle line row (Negate/Posterize/Gradient/Color/Cartoon). */
function ColorsToggleRow(props: {
  label: string;
  checked: boolean;
  onToggle: () => void;
}): ReactElement {
  return (
    <div className="video-row video-row--toggle" data-row="toggle">
      <span className="video-row__label">{props.label}</span>
      <span className="video-row__spacer" aria-hidden="true" />
      <Toggle checked={props.checked} onChange={props.onToggle} aria-label={props.label} />
    </div>
  );
}

/** The Colors tool card — mountable as-is (tools dialog, effects area). */
export function ColorsTool({
  defaultOpen = false,
  id: idProp,
}: VideoToolProps): ReactElement {
  const m = useVideoMsg();
  const [extractionHex, setExtractionHex] = useState(FRAME_EXTRACTION_COLOR);
  const [thresholdHex, setThresholdHex] = useState(FRAME_THRESHOLD_COLOR);
  const [toggles, setToggles] = useState(ALL_OFF);
  const [sliders, setSliders] = useState<Record<string, number>>(() =>
    Object.fromEntries(COLOR_SLIDERS.map((row) => [row.id, row.initial])),
  );
  const [gradientMode, setGradientMode] = useState("gradient");
  const generatedId = useId();
  const id = idProp ?? `colors-${generatedId}`;
  // w26.i08 (h15-P1): the sub-dropdown's list placement is clip-aware — the
  // shared Dropdown's flip (w26.i16 container measurement) lands it inside
  // the tools-dialog body at both contract viewports; the hook only adds
  // the viewport escape/cap for states where no body side fits.
  const gradientFlipRef = useDropdownScrollportFlip<HTMLDivElement>();

  const flip = (toggle: ColorsToggleId): void => {
    setToggles((prev) => ({ ...prev, [toggle]: !prev[toggle] }));
  };

  return (
    <VideoCard
      tool="colors"
      nameKey="customize.panel.video-colors"
      pendingCommand="video.set-colors"
      defaultOpen={defaultOpen}
      id={id}
    >
      <ColorsHexRow
        label={m(FRAME_TOGGLES.extraction)}
        hex={extractionHex}
        onHex={setExtractionHex}
        checked={toggles.extraction}
        onToggle={() => {
          flip("extraction");
        }}
        inputId={`${id}-extraction-hex`}
      />
      {/* w26.i08: per-separator modifiers — the frame's varied cadence
         (video.css pins each gap; the former uniform 13px rhythm compressed
         the lower half up to −26px — h15-P2/g17-P2-1). */}
      <div
        className="video-sep video-sep--1"
        role="separator"
        aria-orientation="horizontal"
      />
      <ColorsHexRow
        label={m(FRAME_TOGGLES.threshold)}
        hex={thresholdHex}
        onHex={setThresholdHex}
        checked={toggles.threshold}
        onToggle={() => {
          flip("threshold");
        }}
        inputId={`${id}-threshold-hex`}
      />
      <VideoSliderRow
        row={SATURATION_ROW}
        value={sliders.saturation ?? SATURATION_ROW.initial}
        onChange={(next) => {
          setSliders((prev) => ({ ...prev, saturation: next }));
        }}
      />
      <VideoSliderRow
        row={SIMILARITY_ROW}
        value={sliders.similarity ?? SIMILARITY_ROW.initial}
        onChange={(next) => {
          setSliders((prev) => ({ ...prev, similarity: next }));
        }}
      />
      <div
        className="video-sep video-sep--2"
        role="separator"
        aria-orientation="horizontal"
      />
      <ColorsToggleRow
        label={m(FRAME_TOGGLES.negate)}
        checked={toggles.negate}
        onToggle={() => {
          flip("negate");
        }}
      />
      <div
        className="video-sep video-sep--3"
        role="separator"
        aria-orientation="horizontal"
      />
      <ColorsToggleRow
        label={m(FRAME_TOGGLES.posterize)}
        checked={toggles.posterize}
        onToggle={() => {
          flip("posterize");
        }}
      />
      <div
        className="video-sep video-sep--4"
        role="separator"
        aria-orientation="horizontal"
      />
      <ColorsToggleRow
        label={m(FRAME_TOGGLES.gradient)}
        checked={toggles.gradient}
        onToggle={() => {
          flip("gradient");
        }}
      />
      <div className="video-subdropdown" ref={gradientFlipRef}>
        <Dropdown
          options={GRADIENT_MODES}
          value={gradientMode}
          onChange={setGradientMode}
          label={m("player.tools.gradient.mode")}
          resolveLabel={m}
        />
      </div>
      <ColorsToggleRow
        label={m(FRAME_TOGGLES.color)}
        checked={toggles.color}
        onToggle={() => {
          flip("color");
        }}
      />
      <ColorsToggleRow
        label={m(FRAME_TOGGLES.cartoon)}
        checked={toggles.cartoon}
        onToggle={() => {
          flip("cartoon");
        }}
      />
      <div
        className="video-sep video-sep--5"
        role="separator"
        aria-orientation="horizontal"
      />
      <VideoSliderRow
        row={SEPIA_ROW}
        value={sliders.sepia ?? SEPIA_ROW.initial}
        onChange={(next) => {
          setSliders((prev) => ({ ...prev, sepia: next }));
        }}
      />
    </VideoCard>
  );
}
