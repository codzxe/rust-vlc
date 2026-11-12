// Card 4.C3 — MINIMAL engine-state bridge (context stand-in).
//
// RECONCILIATION NOTE: card 4.C2 owns ui/src/screens/player/context.tsx (the
// canonical player session context). It did not exist when this card
// started, so this file provides the minimal seam this card needs, consumed
// directly from the command-layer transport (doc 08 §8 — callers under a
// ClientProvider pass `useClient()`; the ClientContext itself is not
// exported, so the transport is an explicit parameter rather than a hidden
// context read). When C2's context lands, this bridge should be deleted and
// the states/ components pointed at the canonical context — they only
// consume `PlayerEngineView`, so the swap is one provider change. Do NOT
// grow this file. [Superseded by w26.f10 (e03 F3): the bridge is now the
// remount-surviving engine/session view — see "remount seam" below.]
//
// Honesty rules (doc 05 §1 C4/C5/C6): live playback state comes ONLY from
// `session.state-changed` events; progress comes only from the engine's
// `session.opening-progress` / `session.buffering` events (f32 0.0..=1.0,
// doc 05 §8 event table); no timer-based or optimistic values are ever
// synthesized. Absent progress events → `null` → indeterminate UI, never a
// fabricated percentage.
//
// REMOUNT SEAM (w26.f10, e03 F3 — "engine truth lost on player-route
// remount"): the player route remounts its whole provider tree on every
// navigation (PlayerScreen mounts PlayerControlsProvider inside the route
// element), and while it is unmounted nobody subscribes to the engine —
// transitions that happen away are lost, so a pure event-reducer restarts
// at `idle` and the empty hero renders over still-playing media. Two
// engine-truth mechanisms close that gap, in precedence order:
//   1. LIVE events — `session.state-changed` this mount always wins;
//   2. a mount-time `system.get-state-snapshot` query completion (the same
//      truth source the canonical C2 context restores from on mount —
//      context.tsx `case "state-snapshot"`) seeds the session state;
// plus a per-transport last-known-view cache that seeds the FIRST render of
// a remount (no idle flash before the snapshot lands). The cache is keyed
// per transport instance (WeakMap): a fresh transport (tests, host swap)
// honestly starts from `idle`; the app-root singleton survives route
// remounts. A late snapshot response never overrides live events; probe
// failures (transport unavailable, `feature-not-exposed` on scaffold
// transports) leave the honest last-known view untouched.

import { useEffect, useReducer } from "react";
import type { CommandTransport, CommandRequest, CommandResponse, Event } from "../../../client/transport";
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

/**
 * Per-transport last-known view (the remount cache). WeakMap so the cache
 * lives exactly as long as the transport instance it describes — engine
 * truth observed through THIS wire, never shared across wires.
 */
const lastEngineViews = new WeakMap<CommandTransport, PlayerEngineView>();

/**
 * requestId for the mount-time snapshot probe. Read-only query, nothing
 * correlates on the id client-side (responses resolve their own promise);
 * 0 stays below the canonical C2 context's incrementing 1..n range.
 */
const SNAPSHOT_REQUEST_ID = 0;

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function applyEvent(view: PlayerEngineView, event: Event): PlayerEngineView {
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

/** Reducer state: the view + whether live session truth arrived this mount. */
interface BridgeState {
  readonly view: PlayerEngineView;
  /**
   * True once a `session.state-changed` fired this mount — at least as
   * fresh as any snapshot query in flight, so the seed backs off.
   */
  readonly live: boolean;
}

type BridgeAction =
  | { readonly type: "event"; readonly event: Event }
  | { readonly type: "snapshot-seed"; readonly state: PlaybackState };

function bridgeReducer(state: BridgeState, action: BridgeAction): BridgeState {
  switch (action.type) {
    case "event": {
      const live = state.live || action.event.kind === "session.state-changed";
      const view = applyEvent(state.view, action.event);
      return live === state.live && view === state.view ? state : { live, view };
    }
    case "snapshot-seed": {
      // Engine-truth seed from `system.get-state-snapshot` (query
      // completion — the C2 context's mount source, doc 05 §3). Seeds ONLY
      // the session state: progress/error remain event-only truths. Live
      // events this mount always win; a same-state seed changes nothing.
      if (state.live || state.view.state === action.state) return state;
      const view: PlayerEngineView =
        action.state === "error"
          ? // Entering error without a live payload → honest unknown; a
            // cached payload from the same error session is kept.
            { ...state.view, state: action.state }
          : // Any other transition clears the previous state's transient
            // evidence (same C6 semantics as session.state-changed).
            {
              ...state.view,
              state: action.state,
              openingProgress: null,
              bufferingProgress: null,
              error: null,
            };
      return { live: false, view };
    }
  }
}

function initBridgeState(transport: CommandTransport | null | undefined): BridgeState {
  const cached = transport == null ? undefined : lastEngineViews.get(transport);
  return { live: false, view: cached ?? INITIAL_VIEW };
}

/** Snapshot response shape guard (a completed state-snapshot result). */
function snapshotStateOf(response: CommandResponse): PlaybackState | null {
  const result = response.result;
  if (!("completed" in result) || result.completed.kind !== "state-snapshot") {
    return null;
  }
  return result.completed.snapshot.state;
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
  const [state, dispatch] = useReducer(bridgeReducer, transport, initBridgeState);

  // Live engine events (doc 05 §3) — the source of truth while mounted.
  useEffect(() => {
    if (transport == null) {
      return;
    }
    return transport.subscribe((event) => dispatch({ type: "event", event }));
  }, [transport]);

  // Mount-time engine snapshot (w26.f10 F3): restores the session state a
  // pure event-reducer would lose across remounts (transitions that fired
  // while unmounted) and on first mount against an already-active engine.
  // Failures surface nowhere — the honest last-known view stands.
  useEffect(() => {
    if (transport == null) {
      return;
    }
    let cancelled = false;
    const probe = async (): Promise<void> => {
      const request: CommandRequest = { cmd: "system.get-state-snapshot", requestId: SNAPSHOT_REQUEST_ID };
      try {
        const response = await transport.send(request);
        if (cancelled) return;
        const seeded = snapshotStateOf(response);
        if (seeded !== null) {
          dispatch({ type: "snapshot-seed", state: seeded });
        }
      } catch {
        // Transport unavailable / command not exposed — nothing to seed.
      }
    };
    void probe();
    return () => {
      cancelled = true;
    };
  }, [transport]);

  // Remember the last view per transport — the next mount over the same
  // wire (route remount) starts from engine truth, not from idle.
  useEffect(() => {
    if (transport != null) {
      lastEngineViews.set(transport, state.view);
    }
  }, [transport, state.view]);

  return transport == null ? INITIAL_VIEW : state.view;
}
