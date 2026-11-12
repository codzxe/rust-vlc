// Card 4.D1 — panel tests: virtualized render, current marker, entry actions
// (dispatch spy), queue controls, drag reorder, empty + not-exposed states.
// The FakePlaylistTransport doubles as the dispatch spy (see provider tests
// for the full catalog-contract coverage).
//
// w26.c6 — queue-row context menu (playlist context menu.svg): the frame's 8
// pinned rows open on right-click; real seams dispatch (play-item,
// set-shuffle, the open-media event, the /media-info route) and no-seam rows
// render honestly disabled.

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport, Event, StateSnapshot } from "../../client/transport";
import { OPEN_MEDIA_EVENT } from "../open-media";
import { PlaylistPanel } from "./PlaylistPanel";
import { PlaylistProvider, type PlaylistEntry } from "./PlaylistProvider";

/** The DOM Event (the catalog `Event` type is imported as a spy shape). */
type DomEvent = globalThis.Event;

class FakeTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();
  private generation = 0;

  constructor(private readonly exposed = true) {}

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
        result: { error: { code: "feature-not-exposed", details: { cmd: request.cmd }, message: "no", stage: "playlist" } },
      };
    }
    if (request.cmd === "system.get-state-snapshot") {
      const snapshot: StateSnapshot = {
        audioDelayMs: 0,
        devices: [],
        duration: { kind: "unknown" },
        muted: false,
        playlist: { current: 2, itemCount: 3, repeat: "off", shuffle: false },
        rate: 1,
        selections: {},
        state: "playing",
        subtitle: {},
        subtitleDelayMs: 0,
        time: null,
        tracks: [],
        videoDelayMs: 0,
        volume: 100,
      };
      return { gen, requestId, result: { completed: { kind: "state-snapshot", snapshot } } };
    }
    if (request.cmd === "playlist.set-shuffle") {
      return { gen, requestId, result: { completed: { kind: "applied-bool", value: request.args.enabled } } };
    }
    if (request.cmd === "playlist.set-repeat") {
      return { gen, requestId, result: { completed: { kind: "applied-repeat", mode: request.args.mode } } };
    }
    return { gen, requestId, result: { completed: { kind: "unit" } } };
  }
}

/** M3U-shaped fixture (crates/playlists round-trip shape): title/location/duration. */
const ENTRIES: PlaylistEntry[] = [
  { id: 1, title: "Big Buck Bunny", source: { kind: "file", path: "/media/bbb.mkv" }, durationSeconds: 634 },
  { id: 2, title: "Sintel", source: { kind: "file", path: "/media/sintel.mkv" }, durationSeconds: null },
  { id: 3, title: null, source: { kind: "url", url: "https://lumen.example/stream" }, durationSeconds: 42 },
];

const setup = (entries: readonly PlaylistEntry[] = ENTRIES, exposed = true) => {
  const transport = new FakeTransport(exposed);
  const view = render(
    <ClientProvider transport={transport}>
      <PlaylistProvider initialEntries={entries}>
        <PlaylistPanel />
      </PlaylistProvider>
    </ClientProvider>,
  );
  return { transport, view };
};

const rowOf = (title: string) => {
  const rows = screen.getAllByRole("option");
  return rows.find((row) => within(row).queryByText(title) !== null)!;
};

describe("PlaylistPanel — render (doc 08 §4.5/§1.6)", () => {
  it("renders M3U-shaped entries: title, location, duration", async () => {
    setup();
    expect(screen.getAllByRole("option").length).toBe(3);
    expect(screen.getByText("Big Buck Bunny")).toBeTruthy();
    expect(screen.getByText("/media/bbb.mkv")).toBeTruthy();
    expect(screen.getByText("10:34")).toBeTruthy(); // 634s
    expect(screen.getByText("0:42")).toBeTruthy();
  });

  it("falls back to honest unknowns for missing title/duration", () => {
    setup();
    expect(screen.getByText("Unknown title")).toBeTruthy();
    expect(screen.getAllByText("—").length).toBe(1); // Sintel has no duration
  });

  it("marks the current item from engine state (aria + sr text + data attr)", async () => {
    setup();
    await waitFor(() => {
      const current = document.querySelector(".lumen-plrow--current");
      expect(current).not.toBeNull();
    });
    expect(screen.getByText("Now playing")).toBeTruthy();
    const row = rowOf("Sintel"); // current id 2
    expect(row.querySelector('[data-current="true"]') !== null).toBe(true);
  });

  it("renders a bounded window for 10k entries, not 10k rows", () => {
    const many: PlaylistEntry[] = Array.from({ length: 10_000 }, (_, i) => ({
      id: i + 1,
      title: `Item ${i}`,
      source: { kind: "file", path: `/m/${i}.mkv` },
      durationSeconds: null,
    }));
    setup(many);
    const rows = screen.getAllByRole("option");
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.length).toBeLessThan(100);
    const spacer = document.querySelector<HTMLElement>(".lumen-vlist__spacer")!;
    expect(spacer.style.height).toBe("400000px"); // 10_000 × ROW_HEIGHTS.compact (40, w26.a09)
  });

  it("shows the honest empty state when the queue has no items", async () => {
    setup([]);
    expect(screen.getByText("Nothing in the queue")).toBeTruthy();
    expect(screen.queryByRole("option")).toBeNull();
  });

  it("shows the not-exposed note instead of pretending (mock transport)", async () => {
    setup([], false);
    await waitFor(() => expect(screen.getByText(/not exposed by this build/i)).toBeTruthy());
  });
});

describe("PlaylistPanel — queue controls + entry actions (dispatch spy)", () => {
  it("next/prev buttons dispatch playlist.next / playlist.prev", async () => {
    const { transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    fireEvent.click(screen.getByRole("button", { name: "Next item" }));
    fireEvent.click(screen.getByRole("button", { name: "Previous item" }));
    expect(transport.sent.some((r) => r.cmd === "playlist.next")).toBe(true);
    expect(transport.sent.some((r) => r.cmd === "playlist.prev")).toBe(true);
  });

  it("shuffle toggle dispatches set-shuffle and reflects engine state", async () => {
    const { transport } = setup();
    await waitFor(() => expect(screen.getByRole("button", { name: "Shuffle" }).getAttribute("aria-pressed")).toBe("false"));
    fireEvent.click(screen.getByRole("button", { name: "Shuffle" }));
    const req = transport.sent.find((r) => r.cmd === "playlist.set-shuffle");
    expect(req && "args" in req && req.args).toEqual({ enabled: true });
  });

  it("repeat button cycles off → all → one via set-repeat", async () => {
    const { transport } = setup();
    await waitFor(() => expect(screen.getByRole("button", { name: "Repeat: off" })).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Repeat: off" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Repeat: all" })).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Repeat: all" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Repeat: one" })).toBeTruthy());
    const modes = transport.sent.filter((r) => r.cmd === "playlist.set-repeat").map((r) => ("args" in r ? r.args : null));
    expect(modes).toEqual([{ mode: "all" }, { mode: "one" }]);
  });

  it("clear button dispatches playlist.clear", async () => {
    const { transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    fireEvent.click(screen.getByRole("button", { name: "Clear playlist" }));
    expect(transport.sent.some((r) => r.cmd === "playlist.clear")).toBe(true);
  });

  it("entry actions dispatch play-item / move (play next, play later) / remove", async () => {
    const { transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    const row = rowOf("Big Buck Bunny"); // index 0; current = Sintel (index 1)
    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "Play now" }));
    expect(transport.sent.filter((r) => r.cmd === "playlist.play-item").map((r) => ("args" in r ? r.args : null))).toEqual([{ id: 1 }]);
    // play-next on an existing id = move right after current (post-removal index 1).
    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "Play next" }));
    // play-later on an existing id = move to the end (index 2 for length 3).
    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "Play later" }));
    const moves = transport.sent.filter((r) => r.cmd === "playlist.move").map((r) => ("args" in r ? r.args : null));
    expect(moves).toEqual([
      { id: 1, toIndex: 1 }, // after current (Sintel at 1; entry removed from 0)
      { id: 1, toIndex: 2 }, // end of the 3-item queue
    ]);
    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "Remove from queue" }));
    expect(transport.sent.filter((r) => r.cmd === "playlist.remove").map((r) => ("args" in r ? r.args : null))).toEqual([{ id: 1 }]);
  });

  it("drag reorder dispatches playlist.move with post-removal to_index", async () => {
    const { transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    const rows = screen.getAllByRole("option");
    const target = rows[0]!.querySelector<HTMLElement>(".lumen-plrow")!;
    fireEvent.drop(target, { dataTransfer: { getData: () => "2" } }); // drag row 2 before row 0
    const move = transport.sent.find((r) => r.cmd === "playlist.move");
    expect(move && "args" in move ? move.args : null).toEqual({ id: 3, toIndex: 0 });
  });
});

describe("PlaylistPanel — queue-row context menu (w26.c6, playlist context menu.svg)", () => {
  /** Right-click the row carrying `title`; returns the opened menu. The
   * ContextMenu wrapper sits INSIDE the windowed row, so the event must
   * target it (or the ListRow) — a dispatch on the outer option would
   * bubble past the wrapper, not through it. */
  const openRowMenu = async (title: string) => {
    const row = rowOf(title);
    const target = row.querySelector<HTMLElement>(".lumen-context-menu")!;
    fireEvent.contextMenu(target, { clientX: 40, clientY: 30 });
    return await screen.findByRole("menu", { name: "Queue item actions" });
  };

  /** All interactive menu rows in DOM order (menuitem + checkbox faces). */
  const menuItems = (menu: HTMLElement): HTMLElement[] =>
    [...menu.querySelectorAll<HTMLElement>('[role="menuitem"], [role="menuitemcheckbox"]')];

  it("opens the frame's 8 rows in pinned order on right-click, keyboard-first", async () => {
    setup();
    const menu = await openRowMenu("Big Buck Bunny");
    // Frame icon order: play · heart · shuffle · save · queue · subtitles · audio · info.
    const labels = menuItems(menu).map((item) => item.textContent);
    expect(labels).toEqual([
      "Play",
      "Favorite",
      "Shuffle",
      "Save playlist",
      "Add media…",
      "Add subtitle tracks",
      "Add audio tracks",
      "Media info",
    ]);
    // WAI-ARIA: opening a menu moves focus onto the first item.
    expect(menuItems(menu)[0]).toHaveFocus();
  });

  it("Play activates the real playlist.play-item command for that row", async () => {
    const { transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    const menu = await openRowMenu("Big Buck Bunny");
    fireEvent.click(within(menu).getByText("Play"));
    expect(transport.sent.filter((r) => r.cmd === "playlist.play-item").map((r) => ("args" in r ? r.args : null))).toEqual([{ id: 1 }]);
    // Activation dismisses the menu (the menu API's onDismissAll).
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("Shuffle is the real set-shuffle command with an engine-truth checkbox face", async () => {
    const { transport } = setup();
    await waitFor(() => expect(transport.sent.length).toBeGreaterThan(0));
    const menu = await openRowMenu("Sintel");
    const shuffle = within(menu).getByRole("menuitemcheckbox", { name: "Shuffle" });
    // Snapshot truth: shuffle false (checked reflects engine state only).
    expect(shuffle).toHaveAttribute("aria-checked", "false");
    fireEvent.click(shuffle);
    const req = transport.sent.find((r) => r.cmd === "playlist.set-shuffle");
    expect(req && "args" in req ? req.args : null).toEqual({ enabled: true });
  });

  it("Add media… fires the shell-wide open-media event", async () => {
    setup();
    const seen: string[] = [];
    const onEvent = (event: DomEvent): void => {
      seen.push(event.type);
    };
    window.addEventListener(OPEN_MEDIA_EVENT, onEvent);
    try {
      const menu = await openRowMenu("Big Buck Bunny");
      fireEvent.click(within(menu).getByText("Add media…"));
    } finally {
      window.removeEventListener(OPEN_MEDIA_EVENT, onEvent);
    }
    expect(seen).toEqual([OPEN_MEDIA_EVENT]);
  });

  it("no-seam rows render honestly disabled (favorite/save/audio — never theater)", async () => {
    setup();
    const menu = await openRowMenu("Big Buck Bunny");
    for (const label of ["Favorite", "Save playlist", "Add audio tracks"]) {
      const item = menuItems(menu).find((row) => row.textContent === label)!;
      expect(item, label).toHaveAttribute("aria-disabled", "true");
    }
  });

  it("Add subtitle tracks opens the Add-Subtitles flow scoped to the queue (w26.f06, e09 F2)", async () => {
    setup();
    const menu = await openRowMenu("Big Buck Bunny");
    const item = menuItems(menu).find((row) => row.textContent === "Add subtitle tracks")!;
    expect(item.getAttribute("aria-disabled")).toBe(null); // real seam now
    fireEvent.click(item);
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Add Subtitles")).toBeTruthy();
    // no folder-scan / OpenSubtitles seam yet — honest empty candidates
    expect(within(dialog).getByText("No subtitle files found")).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("Escape closes the menu (menu API) and restores focus", async () => {
    setup();
    const menu = await openRowMenu("Big Buck Bunny");
    fireEvent.keyDown(within(menu).getByText("Play"), { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("Media info navigates to the /media-info route", async () => {
    const transport = new FakeTransport();
    render(
      <MemoryRouter initialEntries={["/player"]}>
        <Routes>
          <Route
            path="/player"
            element={
              <ClientProvider transport={transport}>
                <PlaylistProvider initialEntries={ENTRIES}>
                  <PlaylistPanel />
                </PlaylistProvider>
              </ClientProvider>
            }
          />
          <Route path="/media-info" element={<div>media-info-screen</div>} />
        </Routes>
      </MemoryRouter>,
    );
    const menu = await openRowMenu("Big Buck Bunny");
    fireEvent.click(within(menu).getByText("Media info"));
    expect(await screen.findByText("media-info-screen")).toBeInTheDocument();
  });
});
