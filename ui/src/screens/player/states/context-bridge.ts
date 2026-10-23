// Card 4.C3 — MINIMAL engine-state bridge (context stand-in).
//
// RECONCILIATION NOTE: card 4.C2 owns ui/src/screens/player/context.ts (the
// canonical player session context). It did not exist when this card
// started, so this file provides the minimal seam this card needs, consumed
// directly from the command-layer transport (doc 08 §8 — callers under a
// ClientProvider pass `useClient()`; the ClientContext itself is not
// exported, so the transport is an explicit parameter rather than a hidden
// context read). When C2's context lands, this bridge should be deleted and
// the states/ components pointed at the canonical context — they only
// consume `PlayerEngineView`, so the swap is one provider change. Do NOT
// grow this file.
//
// Honesty rules (doc 05 §1 C4/C5/C6): playback state comes ONLY from
// `session.state-changed` events; progress comes only from the engine's
// `session.opening-progress` / `session.buffering` events (f32 0.0..=1.0,
// doc 05 §8 event table); no timer-based or optimistic values are ever
// synthesized. Absent progress events → `null` → indeterminate UI, never a
// fabricated percentage.

import { useEffect, useReducer } from "react";
import type { CommandTransport, Event } from "../../../client/transport";
import type { CommandError, PlaybackState } from "../../../generated/commands";

/** Engine-truth view of the session — everything the overlays may render. */
export interface PlayerEngineView {
  /** Current playback state; `idle` until the engine says otherwise. */
  readonly state: PlaybackState;
  /** Last `session.opening-progress` (0..=1); null = none received. */
  readonly openingProgress: number | null;
  /** Last `session.buffering` progress (0..=1); null = none received. */
  readonly bufferingProgress: number | null;
  /** Last fatal `playback.error` payload; null = none received. */
  readonly error: CommandError | null;
}

const INITIAL_VIEW: PlayerEngineView = {
  state: "idle",
  openingProgress: null,
  bufferingProgress: null,
  error: null,
};

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function reduce(view: PlayerEngineView, event: Event): PlayerEngineView {
  switch (event.kind) {
    case "session.state-changed":
      // C4: state comes only from these events. Entering `error` is always
      // accompanied by a preceding fatal `playback.error` (doc 05 §4
      // ordering); leaving any state clears stale progress evidence (C6).
      return event.to === "error"
        ? { ...view, state: event.to }
        : {
            ...view,
            state: event.to,
            openingProgress: null,
            bufferingProgress: null,
            error: null,
          };
    case "session.opening-progress":
      return { ...view, openingProgress: clamp01(event.progress) };
    case "session.buffering":
      return { ...view, bufferingProgress: clamp01(event.progress) };
    case "playback.error":
      // Record the payload; the Error surface renders only while the engine
      // state is `error` (fatal) — non-fatal decode errors stay counters
      // (C5), not overlays. The event carries the error fields inline.
      return {
        ...view,
        error: {
          code: event.code,
          details: event.details,
          message: event.message,
          stage: event.stage,
        },
      };
    default:
      return view;
  }
}

/**
 * Reduce the engine-event stream from a command-layer transport into the
 * overlay view. Callers under a ClientProvider pass `useClient()`'s
 * transport; no transport (scaffold/tests, engine not wired yet) → the
 * honest `idle` view — no engine, no media, no fabricated state.
 */
export function usePlayerEngineView(
  transport?: CommandTransport | null,
): PlayerEngineView {
  const [view, dispatch] = useReducer(reduce, INITIAL_VIEW);

  useEffect(() => {
    if (transport == null) {
      return;
    }
    return transport.subscribe(dispatch);
  }, [transport]);

  return transport == null ? INITIAL_VIEW : view;
}
