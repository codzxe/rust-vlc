// Card 4.F3 — customize panel data model (doc 08 §1.6 + §4 note).
//
// The customize surface is FULLY DATA-DRIVEN per doc 08 §4: "its button
// catalog (±10s/15s, speed, PiP, HDR, …) and display toggles become
// layouts/customize.toml entries referencing commands/UI-actions". This
// module is the consumption side of that contract: a panel definition lists
// fields, each field is bound to a command id, and the renderer shows a
// field ONLY when the engine reports that command as available
// (capability-gated — see capabilities.ts). Unsupported = hidden, never a
// dead control (task card AC; contrast settings' disabled-with-reason).

import type { MessageKey } from "../../../i18n/messages";

/**
 * Args payload for the field's command — the `args` fragment of a
 * CommandRequest. Forward-declared commands (video colors, HDR tone mapping)
 * are not yet in the 1.D4 wire union; their arg keys here are [PROPOSAL]
 * stand-ins that the catalog checksum gate will reconcile on regeneration.
 */
export type CommandArgs = Readonly<Record<string, unknown>>;

/** One selectable value (aspect ratio, crop preset, repeat mode, …). */
export interface CustomizeOption {
  readonly id: string;
  readonly labelKey: MessageKey;
  /** The args payload dispatched when this option is chosen. */
  readonly args: CommandArgs;
}

/** Slider formatting + bounds; `initial` is the neutral DISPLAY position
 * only — committed values come from engine confirmations, never defaults
 * disguised as truth (doc 08 §5). */
export interface SliderSpec {
  readonly min: number;
  readonly max: number;
  readonly step: number;
  readonly initial: number;
  readonly argsFor: (value: number) => CommandArgs;
  /** aria-valuetext / readout formatting. */
  readonly format: (value: number) => string;
}

export type CustomizeControl =
  | { readonly kind: "select"; readonly options: readonly CustomizeOption[] }
  | ({ readonly kind: "slider" } & SliderSpec)
  /** One-shot quick action (seek, stop) with a fixed payload. */
  | { readonly kind: "action"; readonly args: CommandArgs }
  /** Quick action whose payload depends on the last engine-reported bool
   * (shuffle). The button displays no fabricated state — only the dispatched
   * payload flips from the engine truth (null = engine has not reported). */
  | {
      readonly kind: "toggle-action";
      readonly argsFor: (engineValue: boolean | null) => CommandArgs;
    };

/** A single customize field bound to one command id. */
export interface CustomizeField {
  readonly id: string;
  readonly labelKey: MessageKey;
  /** Command id — doubles as the capability gate and the dispatch target. */
  readonly command: string;
  readonly control: CustomizeControl;
}

/** A customize panel (doc 08 §1.6: Video - Colors/Crop/Essential, HDR Options). */
export interface CustomizePanelDef {
  readonly id: string;
  readonly labelKey: MessageKey;
  readonly fields: readonly CustomizeField[];
}
