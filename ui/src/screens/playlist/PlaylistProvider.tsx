// Card 4.D1 — playlist/queue state seam (doc 08 §5 "truthful UI patterns").
//
// OWNERSHIP: ui/src/screens/playlist/** (wave program row 4.D1). Shape rule
// (mirrors 4.C2 context.tsx): only engine-reported state (playlist.changed
// events, command completions, state snapshots) plus honestly-labelled
// unknowns — `null` means "engine has not reported", never a disguised
// default. The command catalog (doc 05 §2.9, ui/src/generated/commands.ts)
// is the contract for every dispatch below.
//
// CATALOG GAP (honest): there is no `playlist.list` command — item DETAILS
// (title/source/duration) cannot be enumerated over the wire yet. Entries
// are seeded via `initialEntries` (tests use M3U-shaped fixtures; the app
// shell will wire `playlist.load` + a real listing source — handoff to card
// 4.H1). `playlist.changed` keeps membership/order/current authoritative:
// ids added by events render with honest unknown fields.

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
  CommandRequest,
  CommandResult,
  Event,
  PlaylistItemId,
  RepeatMode,
  SourceSpec,
} from "../../client/transport";
import { indexOfId, playNextTargetIndex } from "./queue";

/** M3U-shaped entry view-model (doc 03 §4 PlaylistItem; nulls = unknown). */
export interface PlaylistEntry {
  readonly id: PlaylistItemId;
  readonly title: string | null;
  readonly source: SourceSpec | null;
  readonly durationSeconds: number | null;
}

export interface PlaylistState {
  /** Membership/order kept authoritative by `playlist.changed` events. */
  readonly entries: readonly PlaylistEntry[];
  /** From `playlist.changed current-changed` / snapshot; null = none. */
  readonly currentId: PlaylistItemId | null;
  /** `playlist.set-shuffle` state; null = engine has not reported. */
  readonly shuffle: boolean | null;
  /** `playlist.set-repeat` state; null = engine has not reported. */
  readonly repeat: RepeatMode | null;
  /** True once any playlist command answers `feature-not-exposed` — the
   * panel then shows the honest not-connected note instead of pretending. */
  readonly notExposed: boolean;
}

export interface PlaylistActions {
  playItem(id: PlaylistItemId): void;
  next(): void;
  prev(): void;
  skip(n: number): void;
  remove(id: PlaylistItemId): void;
  move(id: PlaylistItemId, toIndex: number): void;
  clear(): void;
  setShuffle(enabled: boolean): void;
  setRepeat(mode: RepeatMode): void;
  /** "Play next" = insertion into the remaining order (doc 03 §810):
   * `playlist.enqueue` then `playlist.move` right after the current item. */
  playNext(source: SourceSpec, title?: string | null): void;
  /** "Play later" = plain append (`playlist.enqueue`). */
  playLater(source: SourceSpec, title?: string | null): void;
  /** `playlist.load` seam (parse lives in the engine/crates/playlists);
   * app-shell file-open wiring is card 4.H1's handoff. */
  load(path: string, append: boolean): void;
}

export type PlaylistContextValue = PlaylistState & PlaylistActions;

const PlaylistContext = createContext<PlaylistContextValue | null>(null);

export function PlaylistProvider({
  children,
  initialEntries = [],
}: {
  children: ReactNode;
  /** Fixture/handoff seed — see the CATALOG GAP note above. */
  initialEntries?: readonly PlaylistEntry[];
}) {
  const transport = useClient();
  const nextRequestId = useRef(0);
  const requestId = useCallback(() => {
    nextRequestId.current += 1;
    return nextRequestId.current;
  }, []);

  const [entries, setEntries] = useState<readonly PlaylistEntry[]>(initialEntries);
  const [currentId, setCurrentId] = useState<PlaylistItemId | null>(null);
  const [shuffle, setShuffleState] = useState<boolean | null>(null);
  const [repeat, setRepeatState] = useState<RepeatMode | null>(null);
  const [notExposed, setNotExposed] = useState(false);

  // w26.k14 (j14 #5) — the LIVE-QUEUE refs. `playNext` must read the queue as
  // of dispatch time (an items-added/moved landing between render and the
  // post-enqueue move used to shift engine indices through a stale render
  // closure), and the actions memo must not depend on the queue at all —
  // every queue change used to rebuild `actions` → the context value → all
  // consumers. All entry/current mutations flow through applyEntries /
  // applyCurrentId, the one place that keeps ref + state in lockstep.
  const entriesRef = useRef<readonly PlaylistEntry[]>(initialEntries);
  const currentIdRef = useRef<PlaylistItemId | null>(null);
  const applyEntries = useCallback(
    (update: (prev: readonly PlaylistEntry[]) => readonly PlaylistEntry[]): void => {
      const next = update(entriesRef.current);
      entriesRef.current = next;
      setEntries(next);
    },
    [],
  );
  const applyCurrentId = useCallback((id: PlaylistItemId | null): void => {
    currentIdRef.current = id;
    setCurrentId(id);
  }, []);

  // Seed engine-reported summary state from the snapshot (doc 05 §2.13).
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const response = await transport.send({
          requestId: requestId(),
          cmd: "system.get-state-snapshot",
        });
        if (cancelled) return;
        if ("completed" in response.result && response.result.completed.kind === "state-snapshot") {
          const summary = response.result.completed.snapshot.playlist;
          if (summary.current !== undefined) applyCurrentId(summary.current ?? null);
          if (summary.shuffle !== undefined) setShuffleState(summary.shuffle);
          if (summary.repeat !== undefined) setRepeatState(summary.repeat);
        } else if ("error" in response.result && response.result.error.code === "feature-not-exposed") {
          setNotExposed(true);
        }
      } catch {
        // Host unreachable — leave honest nulls; panel shows the note.
        setNotExposed(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [transport, requestId, applyCurrentId]);

  // Keep membership/order/current authoritative from `playlist.changed`.
  useEffect(() => {
    return transport.subscribe((event: Event) => {
      if (event.kind !== "playlist.changed") return;
      const change = event.change;
      switch (change.kind) {
        case "items-added":
          applyEntries((prev) => [
            ...prev,
            ...change.ids.map((id) => ({ id, title: null, source: null, durationSeconds: null })),
          ]);
          break;
        case "items-removed":
          applyEntries((prev) => prev.filter((e) => !change.ids.includes(e.id)));
          break;
        case "moved":
          applyEntries((prev) => {
            const next = [...prev];
            const from = next.findIndex((e) => e.id === change.id);
            if (from < 0) return prev;
            const [item] = next.splice(from, 1);
            const to = Math.max(0, Math.min(change.to, next.length));
            next.splice(to, 0, item!);
            return next;
          });
          break;
        case "cleared":
          applyEntries(() => []);
          break;
        case "shuffle-set":
          setShuffleState(change.enabled);
          break;
        case "repeat-set":
          setRepeatState(change.mode);
          break;
        case "current-changed":
          applyCurrentId(change.id);
          break;
      }
    });
  }, [transport, applyEntries, applyCurrentId]);

  const dispatch = useCallback(
    (request: CommandRequest): Promise<CommandResult | null> => {
      return transport.send(request).then(
        (response) => {
          if ("completed" in response.result) return response.result.completed;
          if ("error" in response.result) {
            if (response.result.error.code === "feature-not-exposed") setNotExposed(true);
            return null;
          }
          return null; // accepted — completion arrives via playlist.changed
        },
        () => {
          setNotExposed(true); // host unreachable
          return null;
        },
      );
    },
    [transport],
  );

  const enqueue = useCallback(
    async (source: SourceSpec, title: string | null | undefined): Promise<PlaylistItemId | null> => {
      const result = await dispatch({
        requestId: requestId(),
        cmd: "playlist.enqueue",
        args: { source, ...(title !== undefined && title !== null ? { title } : {}) },
      });
      return result !== null && result.kind === "playlist-item" ? result.id : null;
    },
    [dispatch, requestId],
  );

  const actions = useMemo<PlaylistActions>(
    () => ({
      playItem: (id) => {
        void dispatch({ requestId: requestId(), cmd: "playlist.play-item", args: { id } });
      },
      next: () => {
        void dispatch({ requestId: requestId(), cmd: "playlist.next" });
      },
      prev: () => {
        void dispatch({ requestId: requestId(), cmd: "playlist.prev" });
      },
      skip: (n) => {
        void dispatch({ requestId: requestId(), cmd: "playlist.skip", args: { n } });
      },
      remove: (id) => {
        void dispatch({ requestId: requestId(), cmd: "playlist.remove", args: { id } });
      },
      move: (id, toIndex) => {
        void dispatch({ requestId: requestId(), cmd: "playlist.move", args: { id, toIndex } });
      },
      clear: () => {
        void dispatch({ requestId: requestId(), cmd: "playlist.clear" });
      },
      setShuffle: (enabled) => {
        void (async () => {
          // S-class: the completion IS engine-reported state (doc 05 §2.9).
          const result = await dispatch({ requestId: requestId(), cmd: "playlist.set-shuffle", args: { enabled } });
          if (result !== null && result.kind === "applied-bool") setShuffleState(result.value);
        })();
      },
      setRepeat: (mode) => {
        void (async () => {
          const result = await dispatch({ requestId: requestId(), cmd: "playlist.set-repeat", args: { mode } });
          if (result !== null && result.kind === "applied-repeat") setRepeatState(result.mode);
        })();
      },
      playNext: (source, title) => {
        void (async () => {
          const id = await enqueue(source, title);
          if (id === null) return;
          // w26.k14 (j14 #5): the queue is read through the live ref — an
          // items-added/moved landing between render and this dispatch
          // must shift the math, never a stale render closure.
          const queue = entriesRef.current;
          const currentIndex = indexOfId(
            queue.map((e) => e.id),
            currentIdRef.current ?? -1,
          );
          void dispatch({
            requestId: requestId(),
            cmd: "playlist.move",
            args: { id, toIndex: playNextTargetIndex(currentIndex, queue.length) },
          });
        })();
      },
      playLater: (source, title) => {
        void enqueue(source, title);
      },
      load: (path, append) => {
        void dispatch({ requestId: requestId(), cmd: "playlist.load", args: { path, append } });
      },
    }),
    // w26.k14 (j14 #5): queue state NO LONGER rebuilds the actions — the
    // identities stay stable across playlist.changed events (playNext reads
    // the live refs), so the context value changes only when state does.
    [dispatch, requestId, enqueue],
  );

  const value = useMemo<PlaylistContextValue>(
    () => ({ entries, currentId, shuffle, repeat, notExposed, ...actions }),
    [entries, currentId, shuffle, repeat, notExposed, actions],
  );

  return <PlaylistContext.Provider value={value}>{children}</PlaylistContext.Provider>;
}

export function usePlaylist(): PlaylistContextValue {
  const ctx = useContext(PlaylistContext);
  if (ctx === null) throw new Error("usePlaylist requires a PlaylistProvider ancestor");
  return ctx;
}

/**
 * usePlaylistOptional — the same context read, null when NO engine
 * PlaylistProvider is mounted (w26.f06). The /playlist library page mounts
 * the options layer over LOCAL saved-playlist data (playlistsStore) without
 * an engine queue; `usePlaylist()` would throw there. Null = "no engine
 * scope", the same honest-unknown grammar as the provider's nulls.
 */
export function usePlaylistOptional(): PlaylistContextValue | null {
  return useContext(PlaylistContext);
}
