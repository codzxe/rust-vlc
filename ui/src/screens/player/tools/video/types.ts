// w26.c4 — video tools shared types (ui/src/screens/player/tools/video).
// The four tools share the w26.a20 analysis §D1 tool-card grammar with the
// dynamics lane (§A4–A6): collapsible r6 card ("Expand/Collapse Settings" +
// chevron), label-left rows, full-width tool slider (track 328×16 r6, flat
// tinted fill, 8×24 r4 thumb, optional degree ticks). The slider rows render
// via VideoSliderRow — a 1:1 mirror of the dynamics ToolSliderRow (same DOM,
// same classes) whose labels resolve through THIS lane's useVideoMsg seam:
// the dynamics row's resolver + label-key union are hard-wired to its own
// lane keys, so w26.c4 keys cannot resolve through it (the same reason the
// equalizer lane ships useEqMsg + EqBandRow). The video-specific rows
// (hex+swatch+toggle lines, crop number-input pairs, the Gradient
// sub-dropdown, the HDR dropdown) are also this lane's, composed inside a
// VideoCard that reuses the dynamics card classes verbatim.

import type { ReactNode } from "react";
import type { MessageKey } from "../../../../i18n/messages";
import type { keys as laneKeys } from "../../../../i18n/keys/w26.c4";
import type { keys as dialogsLaneKeys } from "../../../../i18n/keys/w26.a18";

/**
 * Label key union: the central catalog (MessageKey) plus this lane's
 * pre-registered keys (w26.c4.ts). The a18 toggle labels are already central
 * (player.tools.expand/collapse-settings) but stay in the union for the same
 * seam-first reason the dynamics lane keeps them.
 */
export type VideoLabelKey =
  | MessageKey
  | keyof typeof laneKeys
  | keyof typeof dialogsLaneKeys;

/** Props every card-shaped video tool accepts (mountable, uncontrolled). */
export interface VideoToolProps {
  /**
   * Initial expanded state. The frames ship every tool card COLLAPSED;
   * `defaultOpen` exists for hosts (harness, effects area) that want the
   * card open on mount.
   * @default false
   */
  readonly defaultOpen?: boolean;
  /** Stable id for the card's aria wiring; generated when omitted. */
  readonly id?: string;
}

/** VideoCard props — the shared collapsible shell with a free row body. */
export interface VideoCardProps extends VideoToolProps {
  /** Stable tool id (data markers: data-tool / data-staged-tool). */
  readonly tool: string;
  /** Tool name key — names the card group (frames carry no visible title). */
  readonly nameKey: VideoLabelKey;
  /**
   * Forward-declared catalog command the rows await (the effects-area
   * data-pending-command marker; e.g. "video.set-colors"). No such command
   * exists yet — the marker keeps the controls honestly staged.
   */
  readonly pendingCommand: string;
  /** The tool's rows, in frame order (free composition, unlike the
   * dynamics card's sliders-only `rows` API). */
  readonly children: ReactNode;
}

/** HdrTool props — the HDR frame is a bare dropdown, not a card. */
export interface HdrToolProps {
  /** Stable id for the section's markers; generated when omitted. */
  readonly id?: string;
}

/**
 * One slider row: label left (value column omitted — both video frames'
 * slider rows are label-only), full-width tool slider below.
 *
 * Ranges are STAGED DISPLAY RANGES: no video-effects engine stage or catalog
 * command exists yet (the effects-area staged pattern), so each range is
 * calibrated to land its default value at the frame's SVG-measured thumb
 * position — the frames are the design authority. Engine ranges replace
 * these when the video effects command layer lands.
 */
export interface VideoSliderRowDef {
  /** Stable row id (state key + data-row marker). */
  readonly id: string;
  /** Row label key (frame label column). */
  readonly labelKey: VideoLabelKey;
  /** Display range minimum. */
  readonly min: number;
  /** Display range maximum. */
  readonly max: number;
  /** Commit step (keyboard arrows, pointer snap). */
  readonly step: number;
  /** Default value — the frame's thumb position. */
  readonly initial: number;
  /** Slider aria-valuetext (the frames show no visible value column). */
  readonly format: (value: number) => string;
  /**
   * Degree-tick count on the slider (data-driven per frame row; omitted =
   * no ticks). Ticks render at i/(n+1) of the track — the frames' evenly
   * stepped degree marks, with the default value sitting exactly on one.
   */
  readonly ticks?: number;
}

/** Slider-row values keyed by row id — all state stays local to the card. */
export type VideoSliderRowState = Readonly<Record<string, number>>;

/** Crop pair axis — the link icon couples the pair per frame row. */
export type CropAxis = "vertical" | "horizontal";
