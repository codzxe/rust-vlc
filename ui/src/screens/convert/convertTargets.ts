// Card 7.E3 (issue #44) — the conversion target table: the pick list is
// the ENGINE's real capability set, nothing else.
//
// Source of truth: `crates/transcode` (wave 23) exposes exactly two
// conversion operations —
//   * record-to-WAV: decode the selected audio track through the host's
//     decoder factory and write uncompressed RIFF/WAVE PCM (identity or
//     lossless sample widening only — a narrowing or int↔float target is a
//     typed refusal in the engine, never a silent quantization);
//   * remux-to-MP4: H.264 video / AAC audio stream copy only — payload
//     bytes verbatim under fresh sample tables; every other codec is a
//     typed refusal in the engine (crate `mp4mux`, `sample_entry_fourcc`).
//
// This table mirrors that set 1:1. Any other target (MP3, MKV, H.265
// transcode, …) would be a capability the engine does not have — the card's
// acceptance criterion is "no capability shown that is not implemented",
// so the picker simply does not offer one. The exhaustive `ConvertTargetId`
// union + `isConvertTargetId` guard keep the set closed at the type level:
// a future target lands here once the engine ships it, and nowhere else.
//
// [PROPOSAL — for the Lead, card 7.E3 item 2] a third target "raw
// elementary stream" (the engine's `SinkSpec::RawEsCopy`, any codec) is
// implemented engine-side but NOT surfaced: it is a byte dump, not a
// container conversion, and showing it beside WAV/MP4 invites the
// "what do I do with a .h264 file" support question. Surfacing it (file/
// pipe ES sink) is a one-row addition here once ratified.

import type { MessageKey } from "../../i18n/messages";

/** The closed set of conversion targets (module docs). */
export type ConvertTargetId = "wav" | "mp4";

/** One pickable target: id + the l10n keys for its label and honest note. */
export interface ConvertTargetDef {
  readonly id: ConvertTargetId;
  readonly labelKey: MessageKey;
  readonly noteKey: MessageKey;
}

/** The REAL capability table — both engine operations, nothing else. */
export const CONVERT_TARGETS: readonly ConvertTargetDef[] = [
  {
    id: "wav",
    labelKey: "convert.target.wav",
    noteKey: "convert.target.wav.note",
  },
  {
    id: "mp4",
    labelKey: "convert.target.mp4",
    noteKey: "convert.target.mp4.note",
  },
];

/** Type guard: is `value` one of the closed target ids? */
export function isConvertTargetId(value: string): value is ConvertTargetId {
  return value === "wav" || value === "mp4";
}

/** The target definition for `id` (total: the wav default for unknown ids —
 * callers gate with `isConvertTargetId` first; the fallback keeps render
 * paths non-throwing). */
export function convertTargetDef(id: ConvertTargetId): ConvertTargetDef {
  return CONVERT_TARGETS.find((target) => target.id === id) ?? CONVERT_TARGETS[0]!;
}
