// Card 4.C3 — error presentation data (doc 05 §1 C1–C8, doc 08 §5.4).
//
// Doc 05 §2 error rules: `message` is redaction-safe display text (V8) —
// rendered verbatim, never re-derived; raw diagnostic chains live in the log
// sink only. `stage` is the typed stage tag; `code` selects the actionable
// hint. These maps are exhaustive `Record`s so a codegen-side enum growth
// breaks the build here rather than rendering a silent blank (doc 05 §6.2
// requires unknown codes to degrade to a generic failure — the UI side of
// that is the `error.action.internal`-style fallback in StateRouter, while
// TS-level exhaustiveness keeps known codes mapped).
import type { MessageKey } from "../../../i18n/messages";
import type { CommandError, ErrorCode, Stage } from "../../../generated/commands";

/** Stage-tag label key for every typed `Stage` (doc 05 §2). */
export const STAGE_MESSAGE_KEYS: Readonly<Record<Stage, MessageKey>> = {
  session: "player.state.error.stage.session",
  input: "player.state.error.stage.input",
  probe: "player.state.error.stage.probe",
  demux: "player.state.error.stage.demux",
  decode: "player.state.error.stage.decode",
  "audio-output": "player.state.error.stage.audio-output",
  "video-output": "player.state.error.stage.video-output",
  subtitle: "player.state.error.stage.subtitle",
  renderer: "player.state.error.stage.renderer",
  playlist: "player.state.error.stage.playlist",
  config: "player.state.error.stage.config",
  plugin: "player.state.error.stage.plugin",
  control: "player.state.error.stage.control",
};

/** Actionable-hint key for every typed `ErrorCode` (doc 08 §5.4: “what the
 * user can do” — every code gets a real hint, no dead ends). */
export const ERROR_ACTION_MESSAGE_KEYS: Readonly<Record<ErrorCode, MessageKey>> = {
  unsupported: "player.state.error.action.unsupported",
  corrupt: "player.state.error.action.corrupt",
  inaccessible: "player.state.error.action.inaccessible",
  network: "player.state.error.action.network",
  decoder: "player.state.error.action.decoder",
  device: "player.state.error.action.device",
  "resource-limit": "player.state.error.action.resource-limit",
  "invalid-argument": "player.state.error.action.invalid-argument",
  "out-of-range": "player.state.error.action.out-of-range",
  "not-found": "player.state.error.action.not-found",
  "not-ready": "player.state.error.action.not-ready",
  "not-seekable": "player.state.error.action.not-seekable",
  canceled: "player.state.error.action.canceled",
  "feature-not-exposed": "player.state.error.action.feature-not-exposed",
  "unsupported-for-host": "player.state.error.action.unsupported-for-host",
  "permission-denied": "player.state.error.action.permission-denied",
  io: "player.state.error.action.io",
  internal: "player.state.error.action.internal",
};

/** Error codes that route to the dedicated Unsupported surface (doc 08 §5.3:
 * typed `unsupported` error → distinct surface naming format info from the
 * error payload, “no playback attempted” wording). */
export const UNSUPPORTED_ERROR_CODES: readonly ErrorCode[] = ["unsupported", "unsupported-for-host"];

export function isUnsupportedError(error: CommandError): boolean {
  return UNSUPPORTED_ERROR_CODES.includes(error.code);
}

/** Detail keys the Unsupported surface renders (doc 05 §2 rule 3: `details`
 * carries structured hints like `"format": "matroska"`). Whitelist-only —
 * unknown keys are never surfaced (they may carry host-specific diagnostics
 * that are not display copy). */
export type UnsupportedDetailKey = "format" | "codec";

export const UNSUPPORTED_DETAIL_KEYS: readonly UnsupportedDetailKey[] = ["format", "codec"];

/** Extract the format info the typed error actually carries. Returns only
 * present, non-empty entries — never fabricates a container/codec name. */
export function unsupportedFormatInfo(
  error: CommandError,
): Readonly<Partial<Record<UnsupportedDetailKey, string>>> {
  const info: Partial<Record<UnsupportedDetailKey, string>> = {};
  for (const key of UNSUPPORTED_DETAIL_KEYS) {
    const value = error.details[key];
    if (typeof value === "string" && value.length > 0) {
      info[key] = value;
    }
  }
  return info;
}
