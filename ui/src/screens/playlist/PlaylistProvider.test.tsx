// Card 4.D1 — provider tests against the COMMAND CATALOG contract (doc 05
// §2.9): every action must dispatch exactly the catalog command with the
// catalog args; engine state arrives only via events/snapshot (doc 08 §5).

import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type {
  CommandRequest,
  CommandResponse,
  CommandTransport,
  Event,
  StateSnapshot,
} from "../../client/transport";
import { PlaylistProvider, usePlaylist, type PlaylistEntry } from "./PlaylistProvider";

/** Catalog-shaped fake: implements the playlist.* commands over a tiny
 * model and records every request (dispatch spy). */
class FakePlaylistTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();
  private generation = 0;
  private nextId = 100;

  constructor(
    private readonly exposed = true,
    private readonly snapshotOverrides: Partial<StateSnapshot["playlist"]> = {},
    /** w26.k14 (j14 #5): fires inside the playlist.enqueue handler, BEFORE
     * the completion resolves — exactly an engine event landing between
     * the dispatch and the post-enqueue move. */
    private readonly onEnqueue: (() => void) | undefined = undefined,
  ) {}

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    const requestId = request.requestId;
    const gen = ++this.generation;
    if (!this.exposed) {
      return {
        gen,
        requestId,
        result: {
          error: { code: "feature-not-exposed", details: { cmd: request.cmd }, message: "not exposed", stage: "playlist" },
        },
      };
    }
    switch (request.cmd) {
      case "system.get-state-snapshot":
        return { gen, requestId, result: { completed: { kind: "state-snapshot", snapshot: snapshot(this.snapshotOverrides) } } };
      case "playlist.enqueue":
        this.onEnqueue?.();
        return { gen, requestId, result: { completed: { kind: "playlist-item", id: ++this.nextId } } };
      case "playlist.enqueue-many":
        return {
          gen,
          requestId,
          result: { completed: { kind: "playlist-items", ids: request.args.items.map(() => ++this.nextId) } },
        };
      case "playlist.set-shuffle":
        return { gen, requestId, result: { completed: { kind: "applied-bool", value: request.args.enabled } } };
      case "playlist.set-repeat":
        return { gen, requestId, result: { completed: { kind: "applied-repeat", mode: request.args.mode } } };
      default:
        return { gen, requestId, result: { completed: { kind: "unit" } } };
    }
  }
}

function snapshot(overrides: Partial<StateSnapshot["playlist"]> = {}): StateSnapshot {
  return {
    audioDelayMs: 0,
    devices: [],
    duration: { kind: "unknown" },
    muted: false,
    playlist: { current: 2, itemCount: 3, repeat: "all", shuffle: false, ...overrides },
    rate: 1,
    selections: {},
    state: "idle",
    subtitle: {},
    subtitleDelayMs: 0,
    time: null,
    tracks: [],
    videoDelayMs: 0,
    volume: 100,
  };
}

const ENTRY: PlaylistEntry = {
  id: 1,
  title: "Big Buck Bunny",
  source: { kind: "file", path: "/media/bbb.mkv" },
  durationSeconds: 634,
};
const ENTRIES: readonly PlaylistEntry[] = [
  ENTRY,
  { id: 2, title: "Sintel", source: { kind: "file", path: "/media/sintel.mkv" }, durationSeconds: null },
  { id: 3, title: null, source: { kind: "url", url: "https://lumen.example/stream" }, durationSeconds: 42 },
];

const setup = (entries: readonly PlaylistEntry[] = ENTRIES, exposed = true) => {
  const transport = new FakePlaylistTransport(exposed);
  const hook = renderHook(() => usePlaylist(), {
    wrapper: ({ children }) => (
      <ClientProvider transport={transport}>
        <PlaylistProvider initialEntries={entries}>{children}</PlaylistProvider>
      </ClientProvider>
    ),
  });
  return { transport, ...hook };
};

const lastCmd = <C extends CommandRequest["cmd"]>(transport: FakePlaylistTransport, cmd: C) =>
  [...transport.sent].reverse().find((r): r is Extract<CommandRequest, { cmd: C }> => r.cmd === cmd);

describe("PlaylistProvider — catalog command dispatch (spy assertions)", () => {
  it("seeds engine-reported summary state from the snapshot", async () => {
    const { result } = setup();
    await waitFor(() => expect(result.current.currentId).toBe(2));
    expect(result.current.shuffle).toBe(false);
    expect(result.current.repeat).toBe("all");
    expect(result.current.notExposed).toBe(false);
  });

  it("dispatches playlist.play-item with the stable id", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    act(() => result.current.playItem(3));
    const req = lastCmd(transport, "playlist.play-item");
    expect(req?.args).toEqual({ id: 3 });
  });

  it("dispatches playlist.next / playlist.prev / playlist.skip", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    act(() => result.current.next());
    act(() => result.current.prev());
    act(() => result.current.skip(-2));
    expect(transport.sent.some((r) => r.cmd === "playlist.next")).toBe(true);
    expect(transport.sent.some((r) => r.cmd === "playlist.prev")).toBe(true);
    expect(lastCmd(transport, "playlist.skip")?.args).toEqual({ n: -2 });
  });

  it("dispatches playlist.remove / playlist.clear / playlist.move with catalog args", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    act(() => result.current.remove(1));
    act(() => result.current.clear());
    act(() => result.current.move(3, 0));
    expect(lastCmd(transport, "playlist.remove")?.args).toEqual({ id: 1 });
    expect(transport.sent.some((r) => r.cmd === "playlist.clear")).toBe(true);
    expect(lastCmd(transport, "playlist.move")?.args).toEqual({ id: 3, toIndex: 0 });
  });

  it("dispatches playlist.set-shuffle / playlist.set-repeat", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(result.current.repeat).toBe("all"));
    act(() => result.current.setShuffle(true));
    act(() => result.current.setRepeat("one"));
    expect(lastCmd(transport, "playlist.set-shuffle")?.args).toEqual({ enabled: true });
    expect(lastCmd(transport, "playlist.set-repeat")?.args).toEqual({ mode: "one" });
  });

  it("play-later is a plain append: one playlist.enqueue, no move", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    const source = { kind: "file" as const, path: "/media/new.mkv" };
    act(() => result.current.playLater(source, "New item"));
    await waitFor(() => expect(transport.sent.some((r) => r.cmd === "playlist.enqueue")).toBe(true));
    expect(lastCmd(transport, "playlist.enqueue")?.args).toEqual({ source, title: "New item" });
    expect(transport.sent.filter((r) => r.cmd === "playlist.move")).toHaveLength(0);
  });

  it("play-next is an insertion into the remaining order: enqueue then move after current", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(result.current.currentId).toBe(2)); // index 1
    const source = { kind: "url" as const, url: "https://lumen.example/next" };
    act(() => result.current.playNext(source, null));
    await waitFor(() => expect(transport.sent.some((r) => r.cmd === "playlist.move")).toBe(true));
    // current id 2 sits at index 1 → insertion index 2 (doc 03 §810).
    const move = lastCmd(transport, "playlist.move");
    expect(move?.args.toIndex).toBe(2);
    expect(move?.args.id).toBe(101); // id returned by the (fake) enqueue completion
    expect(lastCmd(transport, "playlist.enqueue")?.args).toEqual({ source });
  });

  it("play-next with no current item appends at the end", async () => {
    const transport = new FakePlaylistTransport(true, { current: null });
    const hook = renderHook(() => usePlaylist(), {
      wrapper: ({ children }) => (
        <ClientProvider transport={transport}>
          <PlaylistProvider initialEntries={ENTRIES}>{children}</PlaylistProvider>
        </ClientProvider>
      ),
    });
    await waitFor(() => expect(hook.result.current.shuffle).toBe(false));
    act(() => hook.result.current.playNext({ kind: "file", path: "/media/late.mkv" }, null));
    await waitFor(() => expect(transport.sent.some((r) => r.cmd === "playlist.move")).toBe(true));
    // No current item → insertion at the end (doc 03: nothing is "next").
    expect(lastCmd(transport, "playlist.move")?.args.toIndex).toBe(3);
  });

  // w26.k14 (j14 #5): the move must be computed from the queue AS OF
  // DISPATCH TIME. A `moved` event landing between the enqueue dispatch and
  // the move (the onEnqueue seam above) shifts the current item's index —
  // the pre-fix render closure computed toIndex 2 (stale index 1); the live
  // ref computes 3 (current id 2 now sits at index 2 of [3, 1, 2]).
  it("play-next reads the queue at dispatch time — a mid-flight moved event shifts the move", async () => {
    const transport = new FakePlaylistTransport(true, { current: 2 }, () => {
      transport.emit({ seq: 1, kind: "playlist.changed", change: { kind: "moved", id: 3, from: 2, to: 0 } });
    });
    const hook = renderHook(() => usePlaylist(), {
      wrapper: ({ children }) => (
        <ClientProvider transport={transport}>
          <PlaylistProvider initialEntries={ENTRIES}>{children}</PlaylistProvider>
        </ClientProvider>
      ),
    });
    await waitFor(() => expect(hook.result.current.currentId).toBe(2));
    act(() => hook.result.current.playNext({ kind: "file", path: "/media/next.mkv" }, null));
    await waitFor(() => expect(transport.sent.some((r) => r.cmd === "playlist.move")).toBe(true));
    expect(lastCmd(transport, "playlist.move")?.args.toIndex).toBe(3);
  });

  // w26.k14 (j14 #5): the actions used to rebuild on every queue change
  // (the memo depended on entries/currentId), churning the context value
  // for all consumers. The identities must now survive playlist.changed.
  it("action identities survive queue changes — no context churn", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(result.current.currentId).toBe(2));
    const { playNext, playLater, move, playItem } = result.current;
    act(() => transport.emit({ seq: 1, kind: "playlist.changed", change: { kind: "items-added", ids: [7] } }));
    await waitFor(() => expect(result.current.entries).toHaveLength(4));
    expect(result.current.playNext).toBe(playNext);
    expect(result.current.playLater).toBe(playLater);
    expect(result.current.move).toBe(move);
    expect(result.current.playItem).toBe(playItem);
  });

  it("dispatches playlist.load as the M3U seam (path + append)", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    act(() => result.current.load("/media/queue.m3u", true));
    expect(lastCmd(transport, "playlist.load")?.args).toEqual({ path: "/media/queue.m3u", append: true });
  });
});

describe("PlaylistProvider — engine events keep state authoritative", () => {
  it("applies items-added with honest unknown fields", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(result.current.entries).toHaveLength(3));
    act(() => transport.emit({ seq: 1, kind: "playlist.changed", change: { kind: "items-added", ids: [7, 8] } }));
    expect(result.current.entries.map((e) => e.id)).toEqual([1, 2, 3, 7, 8]);
    expect(result.current.entries[3]).toMatchObject({ title: null, source: null, durationSeconds: null });
  });

  it("applies items-removed, moved and cleared", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(result.current.entries).toHaveLength(3));
    act(() => transport.emit({ seq: 1, kind: "playlist.changed", change: { kind: "items-removed", ids: [1] } }));
    expect(result.current.entries.map((e) => e.id)).toEqual([2, 3]);
    act(() => transport.emit({ seq: 2, kind: "playlist.changed", change: { kind: "moved", id: 3, from: 1, to: 0 } }));
    expect(result.current.entries.map((e) => e.id)).toEqual([3, 2]);
    act(() => transport.emit({ seq: 3, kind: "playlist.changed", change: { kind: "cleared" } }));
    expect(result.current.entries).toHaveLength(0);
  });

  it("applies shuffle-set / repeat-set / current-changed", async () => {
    const { result, transport } = setup();
    await waitFor(() => expect(result.current.currentId).toBe(2));
    act(() => transport.emit({ seq: 1, kind: "playlist.changed", change: { kind: "shuffle-set", enabled: true } }));
    act(() => transport.emit({ seq: 2, kind: "playlist.changed", change: { kind: "repeat-set", mode: "one" } }));
    act(() => transport.emit({ seq: 3, kind: "playlist.changed", change: { kind: "current-changed", id: 3 } }));
    expect(result.current.shuffle).toBe(true);
    expect(result.current.repeat).toBe("one");
    expect(result.current.currentId).toBe(3);
  });

  it("flags feature-not-exposed honestly instead of inventing state", async () => {
    const { result } = setup(ENTRIES, false);
    await waitFor(() => expect(result.current.notExposed).toBe(true));
    // Seeded fixture entries still render (the CATALOG GAP seam), but the
    // engine-reported flags stay null — never faked defaults.
    expect(result.current.shuffle).toBeNull();
    expect(result.current.repeat).toBeNull();
    expect(result.current.currentId).toBeNull();
    expect(result.current.entries).toHaveLength(3);
  });
});
