// w26.gfixA (issue #62) — the customize MODAL tests. The rail-era contracts
// that survive the frame rebuild, re-pinned to the new anatomy:
//  * capability gating: engine-bound tiles render only when listed;
//    UI-owned toolbar items ALWAYS render (the frame's box is the 31-tile
//    catalog, not a capability report — w26.g07-P1-2's split);
//  * tile-command bindings (dispatch spies on the transport);
//  * the frame catalog: the frames' 31-tile inventory + reading order;
//  * the edit model: in-box reorder, drop-on-band places (frames -6/-7),
//    band-item click/drop-into-box removes, Delete key removes;
//  * Save persists the arrangement (localStorage, the variant-seam
//    pattern) and dims while clean (frame 0); Cancel reverts.
import { act, fireEvent, render, renderHook, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import type {
  CommandRequest,
  CommandResponse,
  CommandResult,
  CommandTransport,
  Event,
} from "../../../client/transport";
import type { CommandListing } from "../../../generated/commands";
import { CustomizePanels } from "./CustomizePanels";
import { tileVisible, useCommandCapabilities, visibleTiles } from "./capabilities";
import { CUSTOMIZE_TILES, DEFAULT_TOOLBAR_LAYOUT, tileById } from "./panels";
import {
  TOOLBAR_STORAGE_KEY,
  clearPersistedToolbarLayout,
  parseToolbarLayout,
  toolbarLayoutEqual,
} from "./toolbarLayout";

const listing = (id: string): CommandListing => ({
  id,
  class: "s",
  requires: [],
  since: "0.1.0",
});

const NOT_EXPOSED = (cmd: string) => ({
  error: {
    code: "feature-not-exposed" as const,
    details: { cmd },
    message: "stub: not simulated",
    stage: "control" as const,
  },
});

/**
 * Scriptable transport: `commands` is the capability listing (what
 * `system.list-commands` reports); `complete` scripts command completions;
 * every sent request is recorded for dispatch spies.
 */
class StubTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();

  constructor(
    private readonly commands: readonly CommandListing[],
    private readonly complete?: (request: CommandRequest) => CommandResult | null,
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
    if (request.cmd === "system.list-commands") {
      return {
        gen: 1,
        requestId: request.requestId,
        result: { completed: { kind: "command-listings", commands: [...this.commands] } },
      };
    }
    const completed = this.complete?.(request) ?? null;
    return {
      gen: 1,
      requestId: request.requestId,
      result: completed ? { completed } : NOT_EXPOSED(request.cmd),
    };
  }
}

/** The engine commands the dev shell's demo transport lists. */
const DEMO_COMMANDS = [
  "playback.seek",
  "playback.stop",
  "playback.set-rate",
  "playlist.set-repeat",
  "playlist.set-shuffle",
];

const renderCustomize = (transport: CommandTransport) =>
  render(
    <ClientProvider transport={transport}>
      <CustomizePanels />
    </ClientProvider>,
  );

/** A dataTransfer stand-in that actually stores payloads (jsdom has none). */
const dataTransfer = () => {
  const store = new Map<string, string>();
  return {
    setData: vi.fn((type: string, value: string) => {
      store.set(type, value);
    }),
    getData: (type: string) => store.get(type) ?? "",
    effectAllowed: "",
  };
};

beforeEach(() => {
  clearPersistedToolbarLayout();
});

describe("the frame catalog (Player - customize.svg rows 1–4)", () => {
  it("renders the frames' 31-tile inventory in frame reading order", async () => {
    const stub = new StubTransport(CUSTOMIZE_TILES.flatMap((tile) =>
      tile.command === null ? [] : [listing(tile.command)],
    ));
    renderCustomize(stub);
    const box = await screen.findByRole("list", { name: "Toolbar items" });
    const labels = within(box)
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(labels).toEqual([
      // Row 1 (frame y=318)
      "Spacer", "Divider", "Playback speed", "Forward 10s", "Backward 10s",
      "Forward 15s", "Backward 15s", "Stop", "Repeat",
      // Row 2 (frame y=438)
      "Shuffle", "Record", "Snapshot", "Picture in picture", "HDR", "Zoom",
      "Right sidebar", "Quit VLC", "VLC settings",
      // Row 3 (frame y=558)
      "Go home", "Thumbnail", "File name", "Artist name", "Folder name",
      "Playlist name", "Properties", "Replay", "Chapters",
      // Row 4 (frame y=674)
      "Favorite", "Bookmark", "Bookmarks", "Playlists",
    ]);
  });

  it("carries the modal anatomy: title, verbatim instruction, footer pair", async () => {
    renderCustomize(new StubTransport([]));
    const dialog = await screen.findByRole("dialog", { name: "Customize Player" });
    expect(
      within(dialog).getByText(
        "Drag items around and out of the box below to re-order them. Drag items into the box to remove them from the player.",
      ),
    ).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Save" })).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Close customize panel" })).toBeInTheDocument();
  });

  it("Spacer's well is the frame's empty well (no glyph)", async () => {
    renderCustomize(new StubTransport([]));
    const spacer = (await screen.findByRole("button", { name: "Spacer" })) as HTMLElement;
    expect(spacer.querySelector(".lumen-icon")).toBeNull();
    const divider = screen.getByRole("button", { name: "Divider" }) as HTMLElement;
    expect(divider.querySelector(".lumen-icon")).not.toBeNull();
  });
});

describe("capability gating (AC: available → rendered; not-exposed → hidden)", () => {
  it("renders engine tiles whose commands the engine lists (demo listing)", async () => {
    const stub = new StubTransport(DEMO_COMMANDS.map(listing));
    renderCustomize(stub);
    const box = await screen.findByRole("list", { name: "Toolbar items" });
    // 18 UI-owned + 8 engine-bound = 26 of the frame's 31 (Record,
    // Snapshot, PiP, HDR, Zoom unlisted → honestly hidden).
    const labels = within(box).getAllByRole("button").map((button) => button.textContent);
    expect(labels).toHaveLength(26);
    expect(labels).toContain("Stop");
    expect(labels).toContain("Repeat");
    expect(labels).toContain("Playback speed");
    expect(labels).not.toContain("Record");
    expect(labels).not.toContain("Snapshot");
    expect(labels).not.toContain("Picture in picture");
    expect(labels).not.toContain("HDR");
    expect(labels).not.toContain("Zoom");
  });

  it("UI-owned toolbar items render against the empty scaffold listing too", async () => {
    renderCustomize(new MockTransport());
    const box = await screen.findByRole("list", { name: "Toolbar items" });
    const labels = within(box).getAllByRole("button").map((button) => button.textContent);
    expect(labels).toHaveLength(18);
    expect(labels).toContain("Spacer");
    expect(labels).toContain("Go home");
    expect(labels).toContain("Playlists");
    expect(labels).not.toContain("Stop");
  });

  it("pure helpers: tileVisible encodes the gating matrix (ui-owned always)", () => {
    const caps = { available: new Set<string>(["playback.stop"]), resolved: true };
    const stop = tileById("stop")!;
    const seek = tileById("seek-fwd-10")!;
    const spacer = tileById("spacer")!;
    expect(tileVisible(stop, caps)).toBe(true);
    expect(tileVisible(seek, caps)).toBe(false);
    expect(tileVisible(spacer, caps)).toBe(true); // UI-owned: no gate
    expect(visibleTiles(CUSTOMIZE_TILES, caps)).toHaveLength(19);
  });

  it("useCommandCapabilities resolves ids from system.list-commands", async () => {
    const stub = new StubTransport([listing("playback.stop")]);
    const { result } = renderHook(() => useCommandCapabilities(stub));
    await waitFor(() => expect(result.current.resolved).toBe(true));
    expect(result.current.available.has("playback.stop")).toBe(true);
    expect(result.current.available.has("playback.seek")).toBe(false);
  });
});

describe("tile-command bindings (dispatch spies)", () => {
  it("seek tiles dispatch relative playback.seek targets (±10 s / ±15 s)", async () => {
    const stub = new StubTransport([listing("playback.seek"), listing("playback.stop")]);
    renderCustomize(stub);
    fireEvent.click(await screen.findByRole("button", { name: "Backward 10s" }));
    fireEvent.click(screen.getByRole("button", { name: "Forward 10s" }));
    fireEvent.click(screen.getByRole("button", { name: "Backward 15s" }));
    fireEvent.click(screen.getByRole("button", { name: "Forward 15s" }));
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    await act(async () => {});
    const seeks = stub.sent.filter((request) => request.cmd === "playback.seek");
    expect(
      seeks.map((request) => (request.args as { target: { deltaMs: number } }).target.deltaMs),
    ).toEqual([-10_000, 10_000, -15_000, 15_000]);
    expect(stub.sent.some((request) => request.cmd === "playback.stop")).toBe(true);
  });

  it("UI-owned tiles never dispatch (inert palette items by design)", async () => {
    const stub = new StubTransport(DEMO_COMMANDS.map(listing));
    renderCustomize(stub);
    fireEvent.click(await screen.findByRole("button", { name: "Quit VLC" }));
    fireEvent.click(screen.getByRole("button", { name: "Spacer" }));
    await act(async () => {});
    expect(stub.sent.filter((request) => request.cmd !== "system.list-commands")).toHaveLength(0);
  });

  it("shuffle tile dispatches playlist.set-shuffle and flips on the engine report", async () => {
    const stub = new StubTransport(
      [listing("playlist.set-shuffle")],
      (request) =>
        request.cmd === "playlist.set-shuffle"
          ? { kind: "applied-bool", value: (request.args as { enabled: boolean }).enabled }
          : null,
    );
    renderCustomize(stub);
    const shuffle = await screen.findByRole("button", { name: "Shuffle" });
    expect(shuffle).not.toHaveAttribute("aria-pressed");
    fireEvent.click(shuffle);
    await waitFor(() => expect(shuffle).toHaveAttribute("aria-pressed", "true"));
    fireEvent.click(shuffle);
    await waitFor(() => expect(shuffle).toHaveAttribute("aria-pressed", "false"));
    const sent = stub.sent.filter((request) => request.cmd === "playlist.set-shuffle");
    expect(sent.map((request) => (request.args as { enabled: boolean }).enabled)).toEqual([
      true, false,
    ]);
  });

  it("repeat tile cycles engine-confirmed modes (off → all → one)", async () => {
    const stub = new StubTransport(
      [listing("playlist.set-repeat")],
      (request) =>
        request.cmd === "playlist.set-repeat"
          ? {
              kind: "applied-repeat",
              mode: (request.args as { mode: "off" | "one" | "all" }).mode,
            }
          : null,
    );
    renderCustomize(stub);
    const repeat = await screen.findByRole("button", { name: "Repeat" });
    fireEvent.click(repeat);
    await act(async () => {});
    fireEvent.click(repeat);
    await act(async () => {});
    const sent = stub.sent.filter((request) => request.cmd === "playlist.set-repeat");
    expect(sent.map((request) => (request.args as { mode: string }).mode)).toEqual(["all", "one"]);
  });

  it("playback-speed tile cycles from the engine-confirmed rate (events)", async () => {
    const stub = new StubTransport([listing("playback.set-rate")]);
    renderCustomize(stub);
    const speed = await screen.findByRole("button", { name: "Playback speed" });
    // Engine truth arrives by event: rate 1.5 → the next click dispatches 2.
    act(() => {
      stub.emit({ seq: 1, kind: "playback.rate-changed", rate: 1.5 });
    });
    fireEvent.click(speed);
    await act(async () => {});
    const sent = stub.sent.filter((request) => request.cmd === "playback.set-rate");
    expect(sent.map((request) => (request.args as { rate: number }).rate)).toEqual([2]);
  });
});

describe("the box + the edit band (frames -1…-7)", () => {
  it("frame 0 — open + clean: Save is present but disabled (dimmed), Cancel enabled", async () => {
    renderCustomize(new StubTransport(DEMO_COMMANDS.map(listing)));
    const save = await screen.findByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeEnabled();
  });

  it("the edit band renders frame -1's clean arrangement (no placed items)", async () => {
    renderCustomize(new StubTransport([]));
    const band = await screen.findByRole("group", { name: "Toolbar preview" });
    // Frame -1's clean band: the fixed transport cluster + the long empty
    // slot + the right cluster — NO placed toolbar items until a drop.
    expect(band.querySelectorAll("button")).toHaveLength(0);
    expect(band.querySelector(".lumen-customize-band__slot")).not.toBeNull();
    expect(band.querySelector(".lumen-customize-band__cluster")).not.toBeNull();
  });

  it("frames -1…-5 — dragging a tile in the box reorders; Save enables (dirty)", async () => {
    const stub = new StubTransport([listing("playback.seek")]);
    renderCustomize(stub);
    // Frame order: Forward 10s precedes Backward 10s — dragging Backward
    // ONTO Forward takes Forward's slot (insert-at-target DnD semantics).
    const back = await screen.findByRole("button", { name: "Backward 10s" });
    const fwd = screen.getByRole("button", { name: "Forward 10s" });
    fireEvent.dragStart(back, { dataTransfer: dataTransfer() });
    expect(back).toHaveAttribute("data-dragging");
    fireEvent.dragOver(fwd, { dataTransfer: dataTransfer() });
    expect(fwd).toHaveAttribute("data-drop-target");
    fireEvent.drop(fwd, { dataTransfer: dataTransfer() });
    fireEvent.dragEnd(back, { dataTransfer: dataTransfer() });
    await act(async () => {});
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    const box = screen.getByRole("list", { name: "Toolbar items" });
    const labels = within(box).getAllByRole("button").map((button) => button.textContent);
    expect(labels.indexOf("Backward 10s")).toBeLessThan(labels.indexOf("Forward 10s"));
  });

  it("frames -6/-7 — dropping a tile on the band places it in the toolbar", async () => {
    const stub = new StubTransport([]);
    renderCustomize(stub);
    const favorite = await screen.findByRole("button", { name: "Favorite" });
    const dt = dataTransfer();
    fireEvent.dragStart(favorite, { dataTransfer: dt });
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    fireEvent.dragOver(band, { dataTransfer: dt });
    expect(band).toHaveAttribute("data-drop-active");
    fireEvent.drop(band, { dataTransfer: dt });
    fireEvent.dragEnd(favorite, { dataTransfer: dt });
    await act(async () => {});
    // The placed well appears in the band (frame 7: the favorite in the band).
    expect(
      within(band).getByRole("button", { name: "Remove Favorite from the toolbar" }),
    ).toBeInTheDocument();
    // The box tile carries the in-toolbar hook.
    expect(favorite).toHaveAttribute("data-in-toolbar");
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
  });

  it("dragging a band item INTO the box removes it from the player", async () => {
    const stub = new StubTransport([listing("playback.stop")]);
    renderCustomize(stub);
    const stop = await screen.findByRole("button", { name: "Stop" });
    const dt0 = dataTransfer();
    fireEvent.dragStart(stop, { dataTransfer: dt0 });
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    fireEvent.drop(band, { dataTransfer: dt0 });
    fireEvent.dragEnd(stop, { dataTransfer: dt0 });
    await act(async () => {});
    const stopWell = screen.getByRole("button", { name: "Remove Stop from the toolbar" });
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    const dt = dataTransfer();
    fireEvent.dragStart(stopWell, { dataTransfer: dt });
    const box = screen.getByRole("list", { name: "Toolbar items" });
    fireEvent.drop(box, { dataTransfer: dt });
    fireEvent.dragEnd(stopWell, { dataTransfer: dt });
    await act(async () => {});
    expect(
      screen.queryByRole("button", { name: "Remove Stop from the toolbar" }),
    ).not.toBeInTheDocument();
    // Back to the saved baseline (empty band) — clean again.
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
  });

  it("clicking a placed well removes it (the keyboard-safe path); Delete key too", async () => {
    const stub = new StubTransport([listing("playback.stop")]);
    renderCustomize(stub);
    // Place Stop (drop on the band), then click its well away.
    const stop = await screen.findByRole("button", { name: "Stop" });
    const dt0 = dataTransfer();
    fireEvent.dragStart(stop, { dataTransfer: dt0 });
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    fireEvent.drop(band, { dataTransfer: dt0 });
    fireEvent.dragEnd(stop, { dataTransfer: dt0 });
    await act(async () => {});
    const stopWell = screen.getByRole("button", { name: "Remove Stop from the toolbar" });
    fireEvent.click(stopWell);
    await act(async () => {});
    expect(
      screen.queryByRole("button", { name: "Remove Stop from the toolbar" }),
    ).not.toBeInTheDocument();
    // Keyboard: place Favorite via the band drop, then Delete on its box tile.
    const favorite = screen.getByRole("button", { name: "Favorite" });
    const dt = dataTransfer();
    fireEvent.dragStart(favorite, { dataTransfer: dt });
    fireEvent.drop(band, { dataTransfer: dt });
    fireEvent.dragEnd(favorite, { dataTransfer: dt });
    await act(async () => {});
    expect(favorite).toHaveAttribute("data-in-toolbar");
    fireEvent.keyDown(favorite, { key: "Delete" });
    await act(async () => {});
    expect(favorite).not.toHaveAttribute("data-in-toolbar");
  });

  it("Cancel restores the saved arrangement; Save persists it (localStorage seam)", async () => {
    const stub = new StubTransport([]);
    renderCustomize(stub);
    const favorite = await screen.findByRole("button", { name: "Favorite" });
    const dt = dataTransfer();
    fireEvent.dragStart(favorite, { dataTransfer: dt });
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    fireEvent.drop(band, { dataTransfer: dt });
    fireEvent.dragEnd(favorite, { dataTransfer: dt });
    await act(async () => {});
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    // Cancel reverts the session's edit.
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await act(async () => {});
    expect(
      screen.queryByRole("button", { name: "Remove Favorite from the toolbar" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    // Place again + Save → persisted; a fresh mount reads it back.
    fireEvent.dragStart(favorite, { dataTransfer: dt });
    fireEvent.drop(band, { dataTransfer: dt });
    fireEvent.dragEnd(favorite, { dataTransfer: dt });
    await act(async () => {});
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await act(async () => {});
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    const persisted = globalThis.localStorage?.getItem(TOOLBAR_STORAGE_KEY);
    expect(persisted).toBeDefined();
    expect(parseToolbarLayout(JSON.parse(persisted ?? "null"))?.band).toEqual(["favorite"]);
  });

  it("in-box reorder + Save persists the new catalog order", async () => {
    const stub = new StubTransport([]);
    renderCustomize(stub);
    const spacer = await screen.findByRole("button", { name: "Spacer" });
    fireEvent.keyDown(spacer, { key: "ArrowRight", altKey: true });
    await act(async () => {});
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await act(async () => {});
    const persisted = parseToolbarLayout(
      JSON.parse(globalThis.localStorage?.getItem(TOOLBAR_STORAGE_KEY) ?? "null"),
    );
    expect(persisted?.order[0]).toBe("divider");
    expect(persisted?.band).toEqual([]);
    expect(toolbarLayoutEqual(persisted!, { ...DEFAULT_TOOLBAR_LAYOUT, order: persisted!.order })).toBe(true);
  });
});

describe("toolbar layout persistence helpers", () => {
  it("parseToolbarLayout rejects junk, heals omissions and duplicates", () => {
    expect(parseToolbarLayout(null)).toBeNull();
    expect(parseToolbarLayout("nope")).toBeNull();
    expect(parseToolbarLayout({ order: "x", band: 3 })).toBeNull();
    expect(parseToolbarLayout({ order: [], band: [] })).toBeNull();
    // Unknown ids drop; missing catalog ids append in frame order.
    const healed = parseToolbarLayout({ order: ["stop", "bogus", "stop"], band: ["stop", "nope"] });
    expect(healed?.order[0]).toBe("stop");
    expect(healed?.order).toHaveLength(CUSTOMIZE_TILES.length);
    expect(new Set(healed?.order).size).toBe(healed?.order.length);
    expect(healed?.band).toEqual(["stop"]);
  });

  it("toolbarLayoutEqual compares order + band structurally", () => {
    const a = { order: ["a", "b"], band: ["a"] };
    expect(toolbarLayoutEqual(a, { order: ["a", "b"], band: ["a"] })).toBe(true);
    expect(toolbarLayoutEqual(a, { order: ["b", "a"], band: ["a"] })).toBe(false);
    expect(toolbarLayoutEqual(a, { order: ["a", "b"], band: [] })).toBe(false);
  });
});
