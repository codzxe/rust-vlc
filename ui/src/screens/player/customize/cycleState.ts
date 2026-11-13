// w26.k08 (j08 P2-3): the engine-confirmed CYCLE state shared by the
// customize box (CustomizePanels.tsx) and the real bar (toolbarItems.tsx).
// The box owned this mapping privately since gfixA; the real bar's cycle
// tiles (Playback speed, Zoom) used to re-dispatch default-based args
// forever with no subscription. Both consumers now read the same confirmed
// selections through this module. Engine truth only: completions and
// events map back by args equality — never optimistic (doc 08 §5).
import type { Dispatch, SetStateAction } from "react";
import type { CommandResult, Event } from "../../../client/transport";
import { CUSTOMIZE_TILES } from "./panels";
import type { CommandArgs } from "./types";

/** The cycle-selections state shape both consumers keep. */
export type CycleSelections = Record<string, string>;
export type SetCycleSelections = Dispatch<SetStateAction<CycleSelections>>;

/** Structural equality for plain option-args payloads. */
export function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  const ka = Object.keys(a as Record<string, unknown>);
  const kb = Object.keys(b as Record<string, unknown>);
  if (ka.length !== kb.length) return false;
  return ka.every((key) =>
    deepEqual((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]),
  );
}

/** The cycle tiles' command → args-payload builders (completion mapping). */
export const CYCLE_PAYLOADS: ReadonlyArray<{
  readonly command: string;
  readonly payload: (result: CommandResult) => CommandArgs | null;
}> = [
  { command: "playlist.set-repeat", payload: (r) => (r.kind === "applied-repeat" ? { mode: r.mode } : null) },
  { command: "playback.set-rate", payload: (r) => (r.kind === "applied-rate" ? { rate: r.rate } : null) },
  { command: "video.set-zoom", payload: (r) => (r.kind === "applied-zoom" ? { factor: r.factor } : null) },
];

/** Apply a cycle payload to the matching cycle tile (engine truth only). */
export function applyCyclePayload(
  command: string,
  payload: CommandArgs,
  setCycleSelections: SetCycleSelections,
): void {
  const tile = CUSTOMIZE_TILES.find(
    (candidate) => candidate.command === command && candidate.control.kind === "cycle",
  );
  if (tile === undefined || tile.control.kind !== "cycle") return;
  const option = tile.control.options.find((candidate) => deepEqual(candidate.args, payload));
  if (option !== undefined) {
    setCycleSelections((prev) => ({ ...prev, [tile.id]: option.id }));
  }
}

/** Map a cycle-relevant command completion onto the selections (the cycle
 * half of the box's applyCompletion — applied-bool stays each consumer's
 * own, shuffle-specific concern). */
export function applyCycleCompletion(
  result: CommandResult,
  setCycleSelections: SetCycleSelections,
): void {
  for (const entry of CYCLE_PAYLOADS) {
    const payload = entry.payload(result);
    if (payload !== null) {
      applyCyclePayload(entry.command, payload, setCycleSelections);
      return;
    }
  }
}

/** Map a cycle-relevant engine event (rate change, repeat set) onto the
 * selections — the other source of confirmed truth (doc 05 §3). */
export function applyCycleEvent(event: Event, setCycleSelections: SetCycleSelections): void {
  switch (event.kind) {
    case "playback.rate-changed":
      applyCyclePayload("playback.set-rate", { rate: event.rate }, setCycleSelections);
      break;
    case "playlist.changed":
      if (event.change.kind === "repeat-set") {
        applyCyclePayload("playlist.set-repeat", { mode: event.change.mode }, setCycleSelections);
      }
      break;
    default:
      break;
  }
}
