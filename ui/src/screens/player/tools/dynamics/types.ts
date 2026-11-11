// w26.c2 — dynamics tools shared types (ui/src/screens/player/tools/dynamics).
// The three tools share one card grammar (w26.a20 analysis §A4–A6 / §D1):
// a collapsible r6 card ("Expand/Collapse Settings" + chevron), label-left /
// plain-value-right rows (pitch 76) and a full-width tool slider below each
// row — track 328×16 r6, flat tinted fill, 8×24 r4 thumb, optional degree
// ticks. Data-driven: each tool declares its rows; rendering + local state
// live in DynamicsCard.

import type { MessageKey } from "../../../../i18n/messages";
import type { keys as laneKeys } from "../../../../i18n/keys/w26.c2";
import type { keys as dialogsLaneKeys } from "../../../../i18n/keys/w26.a18";

/**
 * Label key union: the central catalog (MessageKey) plus this lane's
 * pre-registered keys (w26.c2.ts) plus the tools-shell toggle labels
 * pre-registered by w26.a18 (reused, not duplicated).
 */
export type DynamicsLabelKey =
  | MessageKey
  | keyof typeof laneKeys
  | keyof typeof dialogsLaneKeys;

/**
 * One tool row: label left, plain value right, full-width slider below.
 *
 * Ranges are STAGED DISPLAY RANGES: no compressor/spatializer/stereo-widener
 * engine stage or catalog command exists yet (the effects-area staged
 * pattern), so each range is calibrated to land its default value at the
 * frame's measured thumb position — the frames are the design authority.
 * Engine ranges replace these when the DSP command layer lands.
 */
export interface DynamicsRowDef {
  /** Stable row id (state key + data-row marker). */
  readonly id: string;
  /** Row label key (frame label column). */
  readonly labelKey: DynamicsLabelKey;
  /** Display range minimum. */
  readonly min: number;
  /** Display range maximum. */
  readonly max: number;
  /** Commit step (keyboard arrows, pointer snap). */
  readonly step: number;
  /** Default value — the frame's value column. */
  readonly initial: number;
  /** Value-column string + slider aria-valuetext (frame format). */
  readonly format: (value: number) => string;
  /**
   * Degree-tick count on the slider (data-driven per frame row; omitted =
   * no ticks). Ticks render at i/(n+1) of the track — the frame's evenly
   * stepped degree marks, with the default value sitting exactly on one.
   */
  readonly ticks?: number;
}

/** Row values keyed by row id — all state stays local to the card. */
export type DynamicsRowState = Readonly<Record<string, number>>;

/** Props every dynamics tool component accepts (mountable, uncontrolled). */
export interface DynamicsToolProps {
  /**
   * Initial expanded state. The frames ship every tool card COLLAPSED —
   * `defaultOpen` exists for hosts (harness, effects area) that want the
   * card open on mount.
   * @default false
   */
  defaultOpen?: boolean;
  /** Stable id for the card's aria wiring; generated when omitted. */
  id?: string;
}

/** Internal card props (shared renderer; tools compose with their rows). */
export interface DynamicsCardProps extends DynamicsToolProps {
  /** Stable tool id (data markers: data-tool / data-staged-tool). */
  readonly tool: string;
  /** Tool name key — names the card group (frames carry no visible title). */
  readonly nameKey: DynamicsLabelKey;
  /**
   * Forward-declared catalog command the rows await (the effects-area
   * data-pending-command marker; e.g. "audio.set-compressor"). No such
   * command exists yet — the marker keeps the controls honestly staged.
   */
  readonly pendingCommand: string;
  /** The tool's rows, in frame order. */
  readonly rows: readonly DynamicsRowDef[];
}
