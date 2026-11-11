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
// outstanding `playback.play`/`pause` (cleared on `session.state-changed` or
// command completion). Nothing else predicts engine state.

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

/** Position-near-target tolerance for clearing a pending seek [PROPOSAL]. */
const SEEK_CONFIRM_TOLERANCE_SECONDS = 0.75;

const bufferRangesOf = (ranges: readonly TimeRange[]): readonly PlayerBufferRange[] =>
  ranges.map((range) => ({ start: range.start, end: range.end }));

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
    (result: CommandResult): void => {
      switch (result.kind) {
        case "audio-volume":
          patch({ volume: result.volume, engineReported: true });
          break;
        case "applied-bool":
          // Only `audio.set-mute` is dispatched from this provider.
          patch({ muted: result.value, engineReported: true });
          break;
        case "track-list":
          patch({ tracks: result.tracks, engineReported: true });
          break;
        case "applied-selector":
          // Selection truth arrives via `tracks.changed` / re-query.
          break;
        case "current-time":
          patch({
            position: result.position,
            buffered: bufferRangesOf(result.buffered),
            engineReported: true,
          });
          break;
        case "duration":
          patch({
            durationSeconds: result.duration.kind === "known" ? result.duration.time : null,
            engineReported: true,
          });
          break;
        case "media-info":
          patch({
            seekable: result.info.seekable,
            durationSeconds:
              result.info.duration.kind === "known" ? result.info.duration.time : null,
            tracks: result.info.tracks,
            mediaTitle: result.info.title ?? null,
            engineReported: true,
          });
          break;
        case "state-snapshot":
          patch({
            playbackState: result.snapshot.state,
            position: result.snapshot.time ?? 0,
            durationSeconds:
              result.snapshot.duration.kind === "known" ? result.snapshot.duration.time : null,
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
          // "No media open" is a STATE, not a playback failure: the
          // mount-time media.get-info probe on an idle session answers
          // `not-found`, and the stage already renders the honest empty
          // state for it. The error chip stays reserved for real failures
          // (Solstice, 2026-11-08 — demo shell + engine parity).
          const isEmptyProbe =
            request.cmd === "media.get-info" && response.result.error.code === "not-found";
          if (!isEmptyProbe) {
            patch({ lastError: response.result.error });
          }
          return;
        }
        if ("completed" in response.result) {
          applyCompleted(response.result.completed);
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
        // new input cancels the previous one client-side).
        patch({ pendingSeek: seconds });
        void dispatch({
          args: { target: { kind: "absolute", time: seconds } },
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
          setState((prev) => ({
            ...prev,
            position: event.position,
            pendingSeek:
              prev.pendingSeek !== null &&
              Math.abs(event.position - prev.pendingSeek) > SEEK_CONFIRM_TOLERANCE_SECONDS
                ? prev.pendingSeek
                : null,
          }));
          break;
        case "timing.duration-known":
          patch({ durationSeconds: event.duration });
          break;
        case "timing.seek-completed":
          setState((prev) => ({ ...prev, position: event.position, pendingSeek: null }));
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
          const field = selectionFieldFor(event.trackKind as "video" | "audio" | "subtitle");
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
