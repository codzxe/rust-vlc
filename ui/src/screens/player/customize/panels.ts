// Card 4.F3 — the customize panel catalog (doc 08 §1.6 video-tools exports:
// `Video - Colors/Crop/Essential.svg`, `HDR Options.svg`; button catalog per
// §1.6 `player.customize` row). Pure DATA: every field names the command it
// dispatches; capability gating happens in the renderer. When the engine
// does not expose a command the field is hidden, and a panel whose fields
// are ALL unsupported is hidden entirely — no dead controls.
//
// Commands below split into two classes:
//   * in the 1.D4 wire catalog today: video.set-aspect-ratio / set-crop /
//     set-zoom, playback.set-rate / seek / stop, playlist.set-repeat /
//     set-shuffle — bound 1:1 to real commands.
//   * forward-declared (colors, HDR tone mapping): the design (doc 08 §1.6)
//     specifies the panels; the engine/catalog do not carry the commands yet,
//     so these fields stay hidden until a listing exposes them. Their arg
//     keys are [PROPOSAL] pending catalog regeneration.

import type { MessageKey } from "../../../i18n/messages";
import type {
  CustomizeControl,
  CustomizeField,
  CustomizeOption,
  CustomizePanelDef,
} from "./types";

const RATIO_OPTIONS: readonly CustomizeOption[] = [
  { id: "default", labelKey: "customize.option.default", args: { ratio: { kind: "default" } } },
  { id: "1:1", labelKey: "customize.ratio.1-1", args: { ratio: { kind: "ratio", w: 1, h: 1 } } },
  { id: "4:3", labelKey: "customize.ratio.4-3", args: { ratio: { kind: "ratio", w: 4, h: 3 } } },
  { id: "16:9", labelKey: "customize.ratio.16-9", args: { ratio: { kind: "ratio", w: 16, h: 9 } } },
  { id: "16:10", labelKey: "customize.ratio.16-10", args: { ratio: { kind: "ratio", w: 16, h: 10 } } },
  { id: "2.35:1", labelKey: "customize.ratio.2-35-1", args: { ratio: { kind: "ratio", w: 2.35, h: 1 } } },
  { id: "2.39:1", labelKey: "customize.ratio.2-39-1", args: { ratio: { kind: "ratio", w: 2.39, h: 1 } } },
];

const CROP_OPTIONS: readonly CustomizeOption[] = [
  { id: "none", labelKey: "customize.option.none", args: { crop: { kind: "none" } } },
  { id: "1:1", labelKey: "customize.ratio.1-1", args: { crop: { kind: "ratio", w: 1, h: 1 } } },
  { id: "4:3", labelKey: "customize.ratio.4-3", args: { crop: { kind: "ratio", w: 4, h: 3 } } },
  { id: "16:9", labelKey: "customize.ratio.16-9", args: { crop: { kind: "ratio", w: 16, h: 9 } } },
  { id: "16:10", labelKey: "customize.ratio.16-10", args: { crop: { kind: "ratio", w: 16, h: 10 } } },
  { id: "2.35:1", labelKey: "customize.ratio.2-35-1", args: { crop: { kind: "ratio", w: 2.35, h: 1 } } },
  { id: "2.39:1", labelKey: "customize.ratio.2-39-1", args: { crop: { kind: "ratio", w: 2.39, h: 1 } } },
];

const REPEAT_OPTIONS: readonly CustomizeOption[] = [
  { id: "off", labelKey: "customize.repeat.off", args: { mode: "off" } },
  { id: "all", labelKey: "customize.repeat.all", args: { mode: "all" } },
  { id: "one", labelKey: "customize.repeat.one", args: { mode: "one" } },
];

/** The four design panels, in doc 08 §1.6 order: Crop, Colors, Essential, HDR. */
export const CUSTOMIZE_PANELS: readonly CustomizePanelDef[] = [
  {
    id: "crop-aspect",
    labelKey: "customize.panel.crop-aspect",
    fields: [
      {
        id: "aspect-ratio",
        labelKey: "customize.field.aspect",
        command: "video.set-aspect-ratio",
        control: { kind: "select", options: RATIO_OPTIONS },
      },
      {
        id: "crop",
        labelKey: "customize.field.crop",
        command: "video.set-crop",
        control: { kind: "select", options: CROP_OPTIONS },
      },
      {
        id: "zoom",
        labelKey: "customize.field.zoom",
        command: "video.set-zoom",
        control: {
          kind: "slider",
          min: 0.25,
          max: 4,
          step: 0.25,
          initial: 1,
          argsFor: (factor) => ({ factor }),
          format: (value) => `${value.toFixed(2)}x`,
        },
      },
    ],
  },
  {
    id: "video-colors",
    labelKey: "customize.panel.video-colors",
    fields: [
      {
        id: "brightness",
        labelKey: "customize.field.brightness",
        command: "video.set-brightness",
        control: colorSlider(-100, 100, 0),
      },
      {
        id: "contrast",
        labelKey: "customize.field.contrast",
        command: "video.set-contrast",
        control: colorSlider(-100, 100, 0),
      },
      {
        id: "saturation",
        labelKey: "customize.field.saturation",
        command: "video.set-saturation",
        control: colorSlider(-100, 100, 0),
      },
      {
        id: "hue",
        labelKey: "customize.field.hue",
        command: "video.set-hue",
        control: colorSlider(-180, 180, 0),
      },
      {
        id: "gamma",
        labelKey: "customize.field.gamma",
        command: "video.set-gamma",
        control: {
          kind: "slider",
          min: 0.1,
          max: 3,
          step: 0.1,
          initial: 1,
          argsFor: (gamma) => ({ gamma }),
          format: (value) => value.toFixed(2),
        },
      },
    ],
  },
  {
    id: "essential",
    labelKey: "customize.panel.essential",
    fields: [
      {
        id: "speed",
        labelKey: "customize.field.speed",
        command: "playback.set-rate",
        control: {
          kind: "slider",
          min: 0.25,
          max: 4,
          step: 0.25,
          initial: 1,
          argsFor: (rate) => ({ rate }),
          format: (value) => `${value.toFixed(2)}x`,
        },
      },
      seekField("seek-back-60", "customize.action.seek-back-60", -60_000),
      seekField("seek-back-10", "customize.action.seek-back-10", -10_000),
      seekField("seek-fwd-10", "customize.action.seek-fwd-10", 10_000),
      seekField("seek-fwd-60", "customize.action.seek-fwd-60", 60_000),
      {
        id: "stop",
        labelKey: "player.controls.stop",
        command: "playback.stop",
        control: { kind: "action", args: {} },
      },
      {
        id: "repeat",
        labelKey: "customize.action.repeat",
        command: "playlist.set-repeat",
        control: { kind: "select", options: REPEAT_OPTIONS },
      },
      {
        id: "shuffle",
        labelKey: "customize.action.shuffle",
        command: "playlist.set-shuffle",
        control: {
          kind: "toggle-action",
          argsFor: (engineValue) => ({ enabled: !(engineValue ?? false) }),
        },
      },
    ],
  },
  {
    id: "hdr",
    labelKey: "customize.panel.hdr",
    fields: [
      {
        id: "tone-mapping",
        labelKey: "customize.field.tonemap-mode",
        command: "video.set-tone-mapping",
        control: {
          kind: "select",
          options: [
            { id: "clip", labelKey: "customize.tonemap.clip", args: { mode: "clip" } },
            { id: "mobius", labelKey: "customize.tonemap.mobius", args: { mode: "mobius" } },
            { id: "reinhard", labelKey: "customize.tonemap.reinhard", args: { mode: "reinhard" } },
            { id: "hable", labelKey: "customize.tonemap.hable", args: { mode: "hable" } },
          ],
        },
      },
      {
        id: "tone-mapping-strength",
        labelKey: "customize.field.tonemap-strength",
        command: "video.set-tone-mapping-strength",
        control: {
          kind: "slider",
          min: 0,
          max: 100,
          step: 1,
          initial: 50,
          argsFor: (strength) => ({ strength }),
          format: (value) => `${Math.round(value)}%`,
        },
      },
    ],
  },
];

function colorSlider(min: number, max: number, initial: number): CustomizeControl {
  return {
    kind: "slider",
    min,
    max,
    step: 1,
    initial,
    argsFor: (value) => ({ value }),
    format: (value) => String(Math.round(value)),
  };
}

function seekField(id: string, labelKey: MessageKey, deltaMs: number): CustomizeField {
  return {
    id,
    labelKey,
    command: "playback.seek",
    control: { kind: "action", args: { target: { kind: "relative", deltaMs } } },
  };
}

/** Flattened field list — capability/apply helpers scan this. */
export const CUSTOMIZE_FIELDS = CUSTOMIZE_PANELS.flatMap((panel) => panel.fields);
