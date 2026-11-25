// Card 4.C2 — shared player state seam (doc 08 §5 "truthful UI patterns").
//
// OWNERSHIP NOTE (wave program): this file is the player-screen state seam
// created by 4.C2 so 4.C3 (states/**) can consume it without file overlap.
// Shape rule: it carries ONLY engine-reported state (events + command
// completions, doc 05 §3) plus honestly-labelled request-pending fields —
// never optimistic/predicted values.
//
// Accepted vs completed (doc 08 §5): `pendingSeek` marks an outstanding
// `playback.seek` request (set on commit, cleared on `timing.seek-completed`
// / `timing.seek-failed` / position catching up); `pendingToggle` marks an
// outstanding `playback.play`/`pause` (cleared on `session.state-changed`,
// `playback.error`, or — w26.k06, J06-2 — the command answering error /
// the transport failing: a toggle that never completes must not spin
// forever). Nothing else predicts engine state.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useClient } from "../../client/ClientProvider";
import { durationToSeconds, secondsToUs, usToSeconds } from "../../client/wire-time";
import type {
  CommandError,
  CommandTransport,
  CommandRequest,
  CommandResult,
  Event,
  PlaybackState,
  TimeRange,
  TrackId,
  TrackInfo,
  TrackSelector,
  TrackSelections,
} from "../../client/transport";

/** Engine-reported buffered range in seconds (mirrors `TimeRange`). */
export interface PlayerBufferRange {
  readonly start: number;
  readonly end: number;
}

/**
 * The shared player state. `null` fields mean "engine has not reported" —
 * consumers must render honest unknowns, never defaults disguised as truth.
 */
export interface PlayerState {
  /** From `session.state-changed` / snapshot — doc 05 §3. */
  readonly playbackState: PlaybackState;
  /** Engine-reported position in seconds (`timing.time-tick` / queries). */
  readonly position: number;
  /** Seconds; null = unknown/infinite duration. */
  readonly durationSeconds: number | null;
  /** `media.get-info` seekable flag; null = not yet reported. */
  readonly seekable: boolean | null;
  /** Engine-reported buffer ranges; empty when unknown (honest). */
  readonly buffered: readonly PlayerBufferRange[];
  /** 0..1 from engine; null = not yet reported. */
  readonly volume: number | null;
  readonly muted: boolean | null;
  /** Latest `tracks.list` result (also `tracks.list-ready` / snapshot). */
  readonly tracks: readonly TrackInfo[];
  /** Engine-reported media title (`media.get-info` result / `media.info-ready`
   * event); null = not reported (W26: the canvas identity block renders
   * only engine truth — never a fabricated title). Cleared when the session
   * returns to `idle`. */
  readonly mediaTitle: string | null;
  /** Selected track ids per kind; null = engine has not reported. */
  readonly selections: TrackSelections | null;
  /** Outstanding seek target in seconds — the truthful-pending contract. */
  readonly pendingSeek: number | null;
  /** Outstanding play/pause request (doc 08 §5.1 ≤250 ms pending). */
  readonly pendingToggle: boolean;
  /** Last typed error surfaced to the UI (doc 08 §5.4); null = none. */
  readonly lastError: CommandError | null;
  /** True once ANY engine query has succeeded (drives honest disables). */
  readonly engineReported: boolean;
}

export interface PlayerActions {
  play(): void;
  pause(): void;
  togglePlayPause(): void;
  stop(): void;
  /** Dispatch `playback.seek` (absolute) and mark it pending. */
  seek(seconds: number): void;
  /** Dispatch `audio.set-volume`; state updates only on completion. */
  setVolume(value: number): void;
  /** Dispatch `audio.set-mute`; state updates only on completion. */
  setMute(muted: boolean): void;
  /**
   * Dispatch `audio.set-eq` (FX4-A08) — the engine's 3-band Eq3Band +
   * preamp, in dB. The panel's faders stay the editing surface; the
   * engine's typed refusal (e.g. a band beyond ±40 dB) surfaces through
   * `lastError`, never silently.
   */
  setEq(bands: EqSetArgs): void;
  /** Dispatch `tracks.set-{video,audio,subtitle}-track`. */
  selectTrack(kind: "video" | "audio" | "subtitle", selector: TrackSelector): void;
  /** Re-query `tracks.list` (also triggered by menu open / applied-selector). */
  refreshTracks(): void;
  dismissError(): void;
}

export interface PlayerControls {
  readonly state: PlayerState;
  readonly actions: PlayerActions;
}

const INITIAL_STATE: PlayerState = {
  playbackState: "idle",
  position: 0,
  durationSeconds: null,
  seekable: null,
  buffered: [],
  volume: null,
  muted: null,
  tracks: [],
  mediaTitle: null,
  selections: null,
  pendingSeek: null,
  pendingToggle: false,
  lastError: null,
  engineReported: false,
};

const PlayerControlsContext = createContext<PlayerControls | null>(null);

export function usePlayerControls(): PlayerControls {
  const ctx = useContext(PlayerControlsContext);
  if (ctx === null) {
    throw new Error("usePlayerControls requires a PlayerControlsProvider ancestor");
  }
  return ctx;
}

/**
 * Nullable reader for the player state seam — same context, no throw when no
 * `PlayerControlsProvider` is mounted above (w26.f10 F4: hooks that adapt to
 * the player composition's presence, e.g. useChromeAutoHide's legacy
 * fallback outside the player screen). Additive export; the throwing
 * `usePlayerControls` stays the canonical consumer API.
 */
export function usePlayerControlsOrNull(): PlayerControls | null {
  return useContext(PlayerControlsContext);
}

/** Position-near-target tolerance for clearing a pending seek [PROPOSAL]. */
const SEEK_CONFIRM_TOLERANCE_SECONDS = 0.75;

/**
 * FX4-A08 (wave FX4, plan F1.1) — the `audio.set-eq` wire seam.
 *
 * The verb id is the wave contract's (the UI's pre-wave staging markers
 * named it, FX4-A02 routes it to the pipeline's `audio_controls()` in
 * apps/cli/src/host.rs, and FX4-A07's docs pass cites it by name); the
 * payload mirrors the engine setter it feeds — `AudioControls::set_eq(
 * low_db, mid_db, high_db, preamp)` (crates/audio-engine/src/controls.rs)
 * — in the wire's camelCase field convention (`delayMs`/`volume`/`muted`
 * …). The catalog codegen (ui/src/generated/commands.ts — "never
 * hand-edit") carries the command in the union once the wave's central
 * `cargo xtask codegen` refresh lands; until the regenerated union
 * reaches this tree THIS interface is the one typed construction site —
 * the deliberate forward declaration (the k07 lane-key pattern applied
 * to a command payload: one seam, the Lead's regeneration folds it
 * away).
 */
export interface EqSetArgs {
  /** Low-shelf gain in dB (engine corner: 250 Hz). */
  readonly lowDb: number;
  /** Mid-peak gain in dB (engine center: 1 kHz). */
  readonly midDb: number;
  /** High-shelf gain in dB (engine corner: 4 kHz). */
  readonly highDb: number;
  /** Preamp in dB (engine identity: 0). */
  readonly preampDb: number;
}

/**
 * The request the seam issues — shape-identical to the catalog's
 * command-request members (`cmd` + `args` + the provider's requestId).
 */
export interface EqSetRequest {
  readonly cmd: "audio.set-eq";
  readonly args: EqSetArgs;
  readonly requestId: number;
}

/**
 * w26.k06 (J06-2, P1): the commands whose dispatch marks the transport
 * toggle pending. A toggle that answers ERROR (or never reaches the
 * engine — transport failure) never produces the `session.state-changed`
 * that clears the flag, so the failure paths must clear it themselves —
 * otherwise the transport control's pending spinner sticks forever
 * (doc 08 §5.1 "≤250 ms pending" truthful-pending contract).
 */
const isToggleCommand = (cmd: CommandRequest["cmd"]): boolean =>
  cmd === "playback.play" || cmd === "playback.pause" || cmd === "playback.toggle-play-pause";

/**
 * w29.desktop-probe: probe failures that mean "nothing open" or "this
 * host does not serve the probe" are STATES, not playback failures —
 * the error chip stays reserved for real failures (doc 08 §5.4).
 *
 * The mount-time probes meet two idle faces:
 *   - the browser DemoTransport: `media.get-info` → input/not-found
 *     ("no media open" — the shape the original guard covered);
 *   - the REAL desktop engine host (the CLI host behind
 *     `lumen_dispatch`; desktop → lumen-cli, ARCH-5): no snapshot row in
 *     the served table (apps/cli/src/dispatch.rs — doc 05 §5.3's
 *     first-vertical subset answers control/unsupported-for-host), and
 *     `media.get-info` / `media.get-current-time` answer
 *     session/not-ready "no session open" (host.rs — the facts are
 *     session-scoped).
 * For a PASSIVE probe all of these mean the same: no media is open —
 * the honest idle/empty state stands. User-initiated commands
 * (playback.*, seek) keep their typed failures surfaced: a failed
 * action is a failure, never a state.
 */
const isStateProbeRefusal = (
  cmd: CommandRequest["cmd"],
  error: CommandError,
): boolean => {
  switch (cmd) {
    case "media.get-info":
    case "media.get-current-time":
      // Nothing open: the demo's not-found face, the desktop engine's
      // not-ready face (no session ⇒ no facts ⇒ the honest refusal).
      return error.code === "not-found" || error.code === "not-ready";
    case "system.get-state-snapshot":
      // The desktop host's typed "not served" (doc 05 §5.2: capability
      // gaps are typed, not silent — and a typed gap on a passive probe
      // still is not a playback failure; the initial idle view is the
      // honest seed, live events take over).
      return error.code === "unsupported-for-host";
    default:
      return false;
  }
};

const bufferRangesOf = (ranges: readonly TimeRange[]): readonly PlayerBufferRange[] =>
  // w26.k17: wire TimeRanges are µs; the seam documents seconds — cross
  // at the read edge (same fix family as the duration conversions).
  ranges.map((range) => ({ start: usToSeconds(range.start), end: usToSeconds(range.end) }));

const selectionFieldFor = (
  kind: "video" | "audio" | "subtitle",
): keyof TrackSelections => kind;

/**
 * Provider owning all engine communication for the player screen. The
 * transport comes from `ClientProvider` unless explicitly injected (tests).
 */
export function PlayerControlsProvider({
  children,
  transport,
}: {
  children: ReactNode;
  /** Optional transport override (tests); defaults to the client context. */
  transport?: CommandTransport;
}) {
  const client = useClient();
  const wire = transport ?? client;
  const [state, setState] = useState<PlayerState>(INITIAL_STATE);
  const nextRequestId = useRef(0);

  const patch = useCallback((partial: Partial<PlayerState>) => {
    setState((prev) => ({ ...prev, ...partial }));
  }, []);

  /** Apply a command-layer result (completed branch) to engine state. */
  const applyCompleted = useCallback(
    (cmd: CommandRequest["cmd"], result: CommandResult): void => {
      switch (result.kind) {
        case "audio-volume":
          patch({ volume: result.volume, engineReported: true });
          break;
        case "applied-bool":
          // FX4-A08: only `audio.set-mute` maps an applied-bool onto state —
          // `audio.set-eq` (if the host answers one) owns NO state field here,
          // and a future verb answering applied-bool must not smear its
          // value onto `muted` the way a naive kind-match would.
          if (cmd === "audio.set-mute") {
            patch({ muted: result.value, engineReported: true });
          }
          break;
        case "track-list":
          patch({ tracks: result.tracks, engineReported: true });
          break;
        case "applied-selector":
          // Selection truth arrives via `tracks.changed` / re-query.
          break;
        case "current-time":
          // w26.k17 family fix: the µs `time` twin is the truth carrier
          // (the schema's `position` scalar is a 0..1 fraction of duration,
          // redundant here); buffered TimeRanges cross at the edge above.
          patch({
            position: usToSeconds(result.time),
            buffered: bufferRangesOf(result.buffered),
            engineReported: true,
          });
          break;
        case "duration":
          // w26.k17 (family fix landed; was the k06 seam note): the wire
          // `Timestamp` in Duration.time is MICROSECONDS per the generated
          // schema — crossed here into the seconds-documented seam.
          patch({
            durationSeconds: durationToSeconds(result.duration),
            engineReported: true,
          });
          break;
        case "media-info":
          // w26.k17: Duration.time is µs on the wire (see the "duration"
          // case) — crossed at the read edge.
          patch({
            seekable: result.info.seekable,
            durationSeconds: durationToSeconds(result.info.duration),
            tracks: result.info.tracks,
            mediaTitle: result.info.title ?? null,
            engineReported: true,
          });
          break;
        case "state-snapshot":
          // w26.k17: snapshot.time is a µs Timestamp; snapshot.duration
          // crosses through durationToSeconds. Both crossed here.
          patch({
            playbackState: result.snapshot.state,
            position: usToSeconds(result.snapshot.time ?? 0),
            durationSeconds: durationToSeconds(result.snapshot.duration),
            volume: result.snapshot.volume,
            muted: result.snapshot.muted,
            tracks: result.snapshot.tracks,
            selections: result.snapshot.selections,
            engineReported: true,
          });
          break;
        default:
          break;
      }
    },
    [patch],
  );

  /** Dispatch one command; errors are surfaced, never swallowed (§5.4). */
  const dispatch = useCallback(
    async (request: CommandRequest): Promise<void> => {
      try {
        const response = await wire.send(request);
        if ("error" in response.result) {
          // "No media open" and "this host does not serve the probe" are
          // STATES, not playback failures: the mount-time probes on an
          // idle session answer not-found (the demo transport) or
          // not-ready "no session open" (the real desktop engine host),
          // and the desktop host has no snapshot row at all
          // (unsupported-for-host) — the stage already renders the honest
          // empty state for all of them. The error chip stays reserved
          // for real failures (demo shell + engine parity, 2026-11-08;
          // generalized w29.desktop-probe for the desktop cold start).
          if (!isStateProbeRefusal(request.cmd, response.result.error)) {
            patch({ lastError: response.result.error });
          }
          // w26.k06 (J06-2, P1): a failed toggle never completes — clear its
          // pending flag or the transport spinner spins forever.
          if (isToggleCommand(request.cmd)) {
            patch({ pendingToggle: false });
          }
          return;
        }
        if ("completed" in response.result) {
          applyCompleted(request.cmd, response.result.completed);
        }
        // A successfully opened session resets the stale error surface
        // (the previous media's failure no longer applies). session.open
        // answers `accepted` — the opening events follow.
        if (request.cmd === "session.open") {
          patch({ lastError: null });
        }
        // "accepted" — the request_id stays outstanding; pending resolves on
        // the matching engine event (doc 08 §5.1).
      } catch {
        // Transport failure (host unreachable) — typed honest error.
        patch({
          lastError: {
            code: "not-ready",
            details: { cmd: request.cmd },
            message: "command transport unavailable",
            stage: "control",
          },
        });
        // w26.k06 (J06-2, P1): idem — the toggle never reached the engine,
        // so it will never answer with a state change. Clear the spinner.
        if (isToggleCommand(request.cmd)) {
          patch({ pendingToggle: false });
        }
      }
    },
    [applyCompleted, patch, wire],
  );

  const requestId = (): number => {
    nextRequestId.current += 1;
    return nextRequestId.current;
  };

  const actions = useMemo<PlayerActions>(
    () => ({
      play: () => {
        patch({ pendingToggle: true });
        void dispatch({ cmd: "playback.play", requestId: requestId() });
      },
      pause: () => {
        patch({ pendingToggle: true });
        void dispatch({ cmd: "playback.pause", requestId: requestId() });
      },
      togglePlayPause: () => {
        patch({ pendingToggle: true });
        void dispatch({ cmd: "playback.toggle-play-pause", requestId: requestId() });
      },
      stop: () => {
        void dispatch({ cmd: "playback.stop", requestId: requestId() });
      },
      seek: (seconds: number) => {
        // Truthful pending: replace any outstanding request (doc 05 §4 — a
        // new input cancels the previous one client-side). The seam stays
        // seconds; the wire target is µs (w26.k17 family fix).
        patch({ pendingSeek: seconds });
        void dispatch({
          args: { target: { kind: "absolute", time: secondsToUs(seconds) } },
          cmd: "playback.seek",
          requestId: requestId(),
        });
      },
      setVolume: (value: number) => {
        void dispatch({ args: { volume: value }, cmd: "audio.set-volume", requestId: requestId() });
      },
      setMute: (muted: boolean) => {
        void dispatch({ args: { muted }, cmd: "audio.set-mute", requestId: requestId() });
      },
      setEq: (bands: EqSetArgs) => {
        // Same lane as setVolume/setMute — the single transport.send
        // dispatch every working player control rides, with the same
        // monotonic requestId discipline. The cast is the documented
        // forward declaration at the EqSetRequest seam above: the request
        // is shape-identical to the catalog's command requests, carried
        // there until the wave's regenerated union lands.
        const request: EqSetRequest = {
          cmd: "audio.set-eq",
          args: bands,
          requestId: requestId(),
        };
        void dispatch(request as unknown as CommandRequest);
      },
      selectTrack: (kind, selector) => {
        const cmd =
          kind === "video"
            ? "tracks.set-video-track"
            : kind === "audio"
              ? "tracks.set-audio-track"
              : "tracks.set-subtitle-track";
        void dispatch({ args: { selector }, cmd, requestId: requestId() } as CommandRequest);
      },
      refreshTracks: () => {
        void dispatch({ cmd: "tracks.list", requestId: requestId() });
      },
      dismissError: () => {
        patch({ lastError: null });
      },
    }),
    [dispatch, patch],
  );

  // Engine event stream (doc 05 §3) — the only source of "completed" truth.
  useEffect(() => {
    const onEvent = (event: Event): void => {
      switch (event.kind) {
        case "session.state-changed":
          setState((prev) => ({
            ...prev,
            playbackState: event.to,
            pendingToggle: false,
            // A closed session (idle) owns no media title — stale truth
            // never survives into the next session (doc 05 §1 C6).
            mediaTitle: event.to === "idle" ? null : prev.mediaTitle,
            // A state change invalidates an outstanding seek only via the
            // seek events; position itself is not guessed here.
          }));
          break;
        case "session.buffering":
          setState((prev) => ({ ...prev, playbackState: "buffering" }));
          break;
        case "timing.time-tick":
          // w26.k17 family fix (was the k06 seam note): the event's µs
          // `time` twin is read into the seconds-documented `state.position`
          // (the `position` scalar is the schema's 0..1 fraction — redundant
          // for the seam; nothing consumes it alone). k09's bookmarks
          // consumer and its dual-field test seeds stay correct through
          // this read.
          setState((prev) => {
            const position = usToSeconds(event.time);
            return {
              ...prev,
              position,
              pendingSeek:
                prev.pendingSeek !== null &&
                Math.abs(position - prev.pendingSeek) > SEEK_CONFIRM_TOLERANCE_SECONDS
                  ? prev.pendingSeek
                  : null,
            };
          });
          break;
        case "timing.duration-known":
          // w26.k17: the event Timestamp is µs on the wire — crossed here
          // (was the k06 seam note; the family fix landed).
          patch({ durationSeconds: usToSeconds(event.duration) });
          break;
        case "timing.seek-completed":
          // w26.k17: seek-completed.position is a µs Timestamp.
          setState((prev) => ({ ...prev, position: usToSeconds(event.position), pendingSeek: null }));
          break;
        case "timing.seek-failed":
          // Failed seek: surface typed error, snap back to engine position.
          setState((prev) => ({ ...prev, pendingSeek: null, lastError: event.error }));
          break;
        case "tracks.list-ready":
          patch({ tracks: event.tracks, engineReported: true });
          break;
        case "media.info-ready":
          // The engine's info burst after a session open — the honest source
          // for the canvas identity (title). Only engine-reported values
          // land here; a null title stays null (never "Unknown media").
          patch({ mediaTitle: event.info.title ?? null });
          break;
        case "tracks.changed": {
          // w26.k06 (J06-12, P3): TrackChangeKind also carries "data" and
          // "secondary-audio" (generated union, commands.ts) — neither maps
          // to a TrackSelections field, so the old
          // `event.trackKind as "video" | "audio" | "subtitle"` cast wrote
          // stray keys (e.g. `selections.data`). Guard to the three
          // supported kinds; other kinds are honest no-ops here.
          if (
            event.trackKind !== "video" &&
            event.trackKind !== "audio" &&
            event.trackKind !== "subtitle"
          ) {
            break;
          }
          const field = selectionFieldFor(event.trackKind);
          setState((prev) => ({
            ...prev,
            selections:
              prev.selections === null && event.selected === undefined
                ? prev.selections
                : { ...(prev.selections ?? {}), [field]: event.selected ?? null },
          }));
          break;
        }
        case "playback.error":
          setState((prev) => ({
            ...prev,
            lastError: {
              code: event.code,
              details: event.details,
              message: event.message,
              stage: event.stage,
            },
            playbackState: event.fatal ? "error" : prev.playbackState,
            // w26.k06 (J06-2): an async engine failure also resolves an
            // outstanding toggle — the engine transitioned to an error, not
            // to the requested play/pause state; the spinner must not wait
            // for a state change that will never come.
            pendingToggle: false,
          }));
          break;
        default:
          break;
      }
    };
    return wire.subscribe(onEvent);
  }, [patch, wire]);

  // Initial honest state: query the engine snapshot + buffered position +
  // media info (seekable flag). Failures surface as typed errors
  // (MockTransport: feature-not-exposed).
  useEffect(() => {
    void dispatch({ cmd: "system.get-state-snapshot", requestId: requestId() });
    void dispatch({ cmd: "media.get-current-time", requestId: requestId() });
    void dispatch({ cmd: "media.get-info", requestId: requestId() });
  }, [dispatch]);

  const value = useMemo<PlayerControls>(() => ({ state, actions }), [state, actions]);
  return <PlayerControlsContext.Provider value={value}>{children}</PlayerControlsContext.Provider>;
}

/** Track label per doc 07's honest-unknown rule — caller localizes "Unknown". */
export function trackDisplayName(
  track: TrackInfo,
  unknownLabel: string,
  index: number,
): string {
  const name = track.name ?? null;
  if (name !== null && name.length > 0) return name;
  const language = track.language ?? null;
  if (language !== null && language.length > 0) return language;
  // Honest fallback: indexed unknown — never an invented title.
  return `${unknownLabel} ${index + 1}`;
}

/** Convenience: tracks of one kind from the real track list. */
export function tracksOfKind(
  tracks: readonly TrackInfo[],
  kind: "video" | "audio" | "subtitle",
): readonly TrackInfo[] {
  return tracks.filter((track) => track.kind === kind);
}

/**
 * Selected id for a kind. Truth sources, in order: the selections object
 * (snapshot / tracks.changed), else `TrackInfo.isSelected` from the track
 * list; when neither reports a selection the honest answer is "none".
 */
export function selectedTrackId(
  tracks: readonly TrackInfo[],
  selections: TrackSelections | null,
  kind: "video" | "audio" | "subtitle",
): TrackId | null {
  const reported = selections?.[kind];
  if (reported !== undefined) return reported;
  const selected = tracks.find((track) => track.kind === kind && track.isSelected);
  return selected === undefined ? null : selected.id;
}
