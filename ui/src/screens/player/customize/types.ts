// w26.gfixA (issue #62) — the customize surface data model, rebuilt to the
// 9-state frame set `design/figma/Player - customize*.svg`.
//
// THE FRAME MODEL (verified by w26.g07/g08/g09 against every SVG):
//   * the surface is a CENTERED MODAL (1048×655 rx14 #0E0E0E@0.95) — not a
//     tool rail;
//   * the box is the frame's STATIC 31-tile toolbar CATALOG (byte-identical
//     across all 9 frames — rows 1–4, 9 columns), not a membership list;
//   * the LIVE CHROME (control band) is the arrangement being edited: while
//     the modal is open the band renders as dashed edit wells, tiles drag
//     OUT of the box into the band (frames -6/-7) and band items drag back
//     INTO the box ("Drag items into the box to remove them from the
//     player." — the frame's verbatim instruction);
//   * NO adjustment sections exist in any customize frame — the select/
//     slider rows (Crop/Colors/Essential/HDR) were an invention of the
//     rail era; that grammar belongs to the tools dialog
//     (dialogs/ToolsDialog.tsx), which already hosts all four tools.
//
// Data-driven composition (doc 08 §4) is preserved: engine-bound tiles
// name the command they dispatch and render ONLY when the engine's
// `system.list-commands` listing includes that command (capabilities.ts);
// UI-owned toolbar items (Spacer, Go Home, the Name pills, …) have no
// engine command and always render — they are toolbar layout primitives,
// exactly the split w26.g07-P1-2's fix direction calls for.
//
// Truthfulness (doc 08 §5): cycle tiles (Playback speed, Repeat) dispatch
// the NEXT option only from engine-confirmed state — never optimistic.

import type { IconName } from "../../../components/icon/icons";
import type { MessageKey } from "../../../i18n/messages";
import type { keys as a17Keys } from "../../../i18n/keys/w26.a17";
import type { keys as gfixAKeys } from "../../../i18n/keys/w26.gfixA";

/**
 * Tile label keys: central catalog keys, plus the pre-registration lane
 * keys (w26.a17 + this lane's w26.gfixA — the Lead registers them into
 * messages.ts post-wave; the renderer's seam-first resolver finds them
 * either way). Widening past MessageKey is additive.
 */
export type CustomizeLabelKey =
  | MessageKey
  | keyof typeof a17Keys
  | keyof typeof gfixAKeys;

/**
 * Args payload for a tile's command — the `args` fragment of a
 * CommandRequest. Forward-declared commands (PiP, HDR) are not in the 1.D4
 * wire union yet; their arg keys are [PROPOSAL] stand-ins reconciled on
 * catalog regeneration (ui/NEXT-STEPS.md).
 */
export type CommandArgs = Readonly<Record<string, unknown>>;

/** One value of a cycling tile (repeat mode, playback rate, zoom factor). */
export interface CustomizeOption {
  readonly id: string;
  /** Args payload dispatched when this option is cycled to. */
  readonly args: CommandArgs;
}

export type CustomizeControl =
  /** One-shot quick action with a fixed payload (seek, stop, snapshot). */
  | { readonly kind: "action"; readonly args: CommandArgs }
  /** Quick action whose payload flips with the last engine-reported bool
   * (shuffle). Displays no fabricated state — the dispatched payload is
   * derived from engine truth only (null = engine has not reported). */
  | {
      readonly kind: "toggle-action";
      readonly argsFor: (engineValue: boolean | null) => CommandArgs;
    }
  /** Cycling quick action (Playback speed, Repeat, Zoom): each click
   * dispatches the option AFTER the engine-confirmed one (wraps; before
   * any report the `defaultOption` follows). Engine confirmations and
   * events map back by args equality — never optimistic. */
  | {
      readonly kind: "cycle";
      readonly options: readonly CustomizeOption[];
      readonly defaultOption: string;
    }
  /** UI-owned toolbar item (Spacer, Go Home, …): no engine command, no
   * dispatch — its click is inert; the tile exists to be dragged into the
   * toolbar arrangement. */
  | { readonly kind: "ui" };

/** One tile of the frame's catalog (a toolbar item the arrangement edits). */
export interface CustomizeTile {
  readonly id: string;
  readonly labelKey: CustomizeLabelKey;
  /**
   * Command id — doubles as the capability gate and the dispatch target.
   * null = UI-owned toolbar item (always rendered, never dispatches).
   */
  readonly command: string | null;
  readonly control: CustomizeControl;
  /**
   * Palette glyph (the frames draw 18–20px 1.5-stroke icons inside every
   * well — EXCEPT Spacer, whose well is deliberately empty).
   */
  readonly icon?: IconName;
}

/**
 * The saved toolbar arrangement (doc 08 §4: toolbar layout is UI-OWNED
 * state) — what the customize surface edits and persists:
 *   * `order`  — the catalog box's display order ("Drag items around … to
 *                re-order them");
 *   * `band`   — the items placed in the control band (frame -1's clean
 *                baseline carries none; frames -6/-7 show the dragged
 *                tile landing there).
 */
export interface ToolbarLayout {
  readonly order: readonly string[];
  readonly band: readonly string[];
}
