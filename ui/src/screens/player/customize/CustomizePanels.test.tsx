// w26.gfixA (issue #62) — the customize MODAL tests. The rail-era contracts
// that survive the frame rebuild, re-pinned to the new anatomy:
//  * the frame catalog: the STATIC 31-tile inventory renders against ANY
//    listing (h07 defect 4 — the box is a catalog, not a capability report;
//    engine truth governs dispatch, never membership — w26.g07-P1-2's
//    split, updated w26.k08 to match the shipped policy);
//  * tile-command bindings (dispatch spies on the transport);
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
import { CustomizeToolbarItems } from "./toolbarItems";
import { useCommandCapabilities } from "./capabilities";
import { CUSTOMIZE_TILES, CUSTOMIZE_TILE_IDS, DEFAULT_TOOLBAR_LAYOUT } from "./panels";
import { PlayerControlsProvider } from "../context";
import { PlaylistProvider } from "../../playlist";
import {
  TOOLBAR_CHANGED_EVENT,
  TOOLBAR_STORAGE_KEY,
  clearPersistedToolbarLayout,
  parseToolbarLayout,
  persistToolbarLayout,
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
  it("renders the frames' 31-tile inventory in frame reading order (any listing)", async () => {
    // w26.i05 (h07 defect 4): the box is the frame's STATIC catalog — all 31
    // tiles render against ANY capability listing (9/9/9/4 at 9 columns);
    // engine truth still governs dispatch/aria, never membership.
    renderCustomize(new StubTransport([]));
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
    // The 4-row rhythm never collapses: the full 31 (rows 9/9/9/4).
    expect(labels).toHaveLength(31);
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

describe("capability policy (w26.i05: the box is the catalog, not a report)", () => {
  it("renders the full 31 against the demo listing — Record/Snapshot/PiP/HDR/Zoom included", async () => {
    const stub = new StubTransport(DEMO_COMMANDS.map(listing));
    renderCustomize(stub);
    const box = await screen.findByRole("list", { name: "Toolbar items" });
    // h07 defect 4: the demo listing used to hide 5 tiles (26/31, rows
    // collapsed 9/9/8); the frame's inventory renders regardless — the
    // honest no-change on dispatch stays the engine's answer.
    const labels = within(box).getAllByRole("button").map((button) => button.textContent);
    expect(labels).toHaveLength(31);
    expect(labels).toContain("Record");
    expect(labels).toContain("Snapshot");
    expect(labels).toContain("Picture in picture");
    expect(labels).toContain("HDR");
    expect(labels).toContain("Zoom");
  });

  it("the same 31 render against the empty scaffold listing (MockTransport)", async () => {
    renderCustomize(new MockTransport());
    const box = await screen.findByRole("list", { name: "Toolbar items" });
    const labels = within(box).getAllByRole("button").map((button) => button.textContent);
    expect(labels).toHaveLength(31);
    expect(labels).toContain("Spacer");
    expect(labels).toContain("Go home");
    expect(labels).toContain("Playlists");
    expect(labels).toContain("Stop");
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

  it("persistToolbarLayout broadcasts the lumen:toolbar-changed seam (w26.i05)", () => {
    const heard = vi.fn();
    window.addEventListener(TOOLBAR_CHANGED_EVENT, heard);
    try {
      persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["favorite"] });
      expect(heard).toHaveBeenCalledTimes(1);
      expect(globalThis.localStorage?.getItem(TOOLBAR_STORAGE_KEY)).toBeDefined();
    } finally {
      window.removeEventListener(TOOLBAR_CHANGED_EVENT, heard);
    }
  });
});

describe("the per-state band variants (w26.i05, h07 defect 3 — frames -1..-6)", () => {
  it("f0 — clean: no seek row; the long slot carries the live preview + timecodes", async () => {
    renderCustomize(new StubTransport([]));
    const band = await screen.findByRole("group", { name: "Toolbar preview" });
    // No promoted seek row while clean (frame 0 keeps the seek INSIDE the slot).
    expect(band.querySelector(".lumen-customize-band__seekrow")).toBeNull();
    const slot = band.querySelector(".lumen-customize-band__slot")!;
    expect(slot).not.toBeNull();
    // Frame 0's band timecodes (Current Time.svg / Duration.svg values).
    expect(slot.textContent).toContain("00:02:10");
    expect(slot.textContent).toContain("00:25:40");
    expect(slot.querySelector(".lumen-customize-band__seek")).not.toBeNull();
    // One slot, transport cluster at the left anchor.
    expect(band.querySelectorAll(".lumen-customize-band__slot")).toHaveLength(1);
    expect(band.querySelector(".lumen-customize-band__cluster")).not.toBeNull();
  });

  it("f1 — dirty reorder: the seek row promotes ABOVE; the slot is left EMPTY", async () => {
    renderCustomize(new StubTransport([]));
    const spacer = await screen.findByRole("button", { name: "Spacer" });
    fireEvent.keyDown(spacer, { key: "ArrowRight", altKey: true });
    await act(async () => {});
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    const seekrow = band.querySelector(".lumen-customize-band__seekrow");
    expect(seekrow).not.toBeNull();
    // The promoted row carries the frame's timecodes + track; the
    // transport slot is empty (frame -1: 1431×35 @155.5).
    expect(seekrow?.textContent).toContain("00:02:10");
    expect(seekrow?.textContent).toContain("00:25:40");
    const slot = band.querySelector(".lumen-customize-band__slot")!;
    expect(slot.querySelector(".lumen-customize-band__seek")).toBeNull();
    expect(band.querySelectorAll(".lumen-customize-band__slot")).toHaveLength(1);
  });

  it("f2 — one placed: transport centered between split slots", async () => {
    renderCustomize(new StubTransport([]));
    const favorite = await screen.findByRole("button", { name: "Favorite" });
    const dt = dataTransfer();
    fireEvent.dragStart(favorite, { dataTransfer: dt });
    fireEvent.drop(screen.getByRole("group", { name: "Toolbar preview" }), { dataTransfer: dt });
    fireEvent.dragEnd(favorite, { dataTransfer: dt });
    await act(async () => {});
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    expect(band.querySelectorAll(".lumen-customize-band__slot")).toHaveLength(2);
    expect(band.querySelector(".lumen-customize-band__slot--left")).not.toBeNull();
    expect(band.querySelector(".lumen-customize-band__slot--right")).not.toBeNull();
    expect(band.querySelector(".lumen-customize-band__seekrow")).not.toBeNull();
  });

  it("f3 — two placed: the 130×26 well joins the left edge", async () => {
    renderCustomize(new StubTransport([]));
    for (const name of ["Favorite", "Bookmark"]) {
      const tile = screen.getByRole("button", { name });
      const dt = dataTransfer();
      fireEvent.dragStart(tile, { dataTransfer: dt });
      fireEvent.drop(screen.getByRole("group", { name: "Toolbar preview" }), { dataTransfer: dt });
      fireEvent.dragEnd(tile, { dataTransfer: dt });
      await act(async () => {});
    }
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    // Frame -3: the 130×26 flat well joins at the band's LEFT EDGE, before
    // the narrowed (729) left slot — DOM order pins the composition (the
    // old assertion here only checked a class against itself; j08 noted it
    // as mild theater — this is the real structural pin).
    const flat = band.querySelector(".lumen-customize-band__well--flat");
    expect(flat).not.toBeNull();
    const leftSlot = band.querySelector(".lumen-customize-band__slot--left");
    expect(leftSlot).not.toBeNull();
    expect(
      Boolean(flat!.compareDocumentPosition(leftSlot!) & Node.DOCUMENT_POSITION_FOLLOWING),
    ).toBe(true);
  });

  it("f4 — three placed: the topbar well run enters the band", async () => {
    renderCustomize(new StubTransport([]));
    for (const name of ["Favorite", "Bookmark", "Bookmarks"]) {
      const tile = screen.getByRole("button", { name });
      const dt = dataTransfer();
      fireEvent.dragStart(tile, { dataTransfer: dt });
      fireEvent.drop(screen.getByRole("group", { name: "Toolbar preview" }), { dataTransfer: dt });
      fireEvent.dragEnd(tile, { dataTransfer: dt });
      await act(async () => {});
    }
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    const run = band.querySelector(".lumen-customize-band__topbarrun");
    expect(run).not.toBeNull();
    // Frame -4's run: 27 / 27÷mark / 23 / 60 / 130×26.
    expect(run?.querySelectorAll(".lumen-customize-band__well").length).toBe(5);
    expect(run?.querySelector(".lumen-customize-band__well-mark")).not.toBeNull();
  });

  it("f5/f6 — dragging: the ghost pins band-left, the lifted box slot vacates, the cluster shifts", async () => {
    renderCustomize(new StubTransport([]));
    for (const name of ["Favorite", "Bookmark", "Bookmarks"]) {
      const tile = screen.getByRole("button", { name });
      const dt = dataTransfer();
      fireEvent.dragStart(tile, { dataTransfer: dt });
      fireEvent.drop(screen.getByRole("group", { name: "Toolbar preview" }), { dataTransfer: dt });
      fireEvent.dragEnd(tile, { dataTransfer: dt });
      await act(async () => {});
    }
    // Lift the 4th (frame -5/-6's mid-drag).
    const playlists = screen.getByRole("button", { name: "Playlists" });
    fireEvent.dragStart(playlists, { dataTransfer: dataTransfer() });
    await act(async () => {});
    expect(playlists).toHaveAttribute("data-dragging");
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    // The styled ghost (icon-only, pinned before the left slot).
    const ghost = band.querySelector(".lumen-customize-band__ghost");
    expect(ghost).not.toBeNull();
    expect(ghost?.querySelector(".lumen-icon")).not.toBeNull();
    // The right cluster shifts (frame -5) + the far-right 35×23 well.
    expect(band.querySelector(".lumen-customize-band__right")).toHaveAttribute("data-shifted");
    expect(band.querySelector(".lumen-customize-band__well--low")).not.toBeNull();
    fireEvent.dragEnd(playlists, { dataTransfer: dataTransfer() });
    await act(async () => {});
    expect(band.querySelector(".lumen-customize-band__ghost")).toBeNull();
  });

  it("a lifted band item vacates its well (drag back into the box)", async () => {
    renderCustomize(new StubTransport([]));
    const favorite = await screen.findByRole("button", { name: "Favorite" });
    const dt = dataTransfer();
    fireEvent.dragStart(favorite, { dataTransfer: dt });
    fireEvent.drop(screen.getByRole("group", { name: "Toolbar preview" }), { dataTransfer: dt });
    fireEvent.dragEnd(favorite, { dataTransfer: dt });
    await act(async () => {});
    const well = screen.getByRole("button", { name: "Remove Favorite from the toolbar" });
    fireEvent.dragStart(well, { dataTransfer: dataTransfer() });
    await act(async () => {});
    expect(well).toHaveAttribute("data-dragging");
    // The ghost follows the lifted band item too.
    const band = screen.getByRole("group", { name: "Toolbar preview" });
    expect(band.querySelector(".lumen-customize-band__ghost")).not.toBeNull();
    fireEvent.dragEnd(well, { dataTransfer: dataTransfer() });
    await act(async () => {});
    expect(well).not.toHaveAttribute("data-dragging");
  });
});

describe("focus + Escape semantics (w26.i05, h07 defect 2)", () => {
  it("Save keeps focus inside the dialog; Escape still closes (the lumen:close-panel seam)", async () => {
    const heard = vi.fn();
    window.addEventListener("lumen:close-panel", heard);
    try {
      renderCustomize(new StubTransport([]));
      const favorite = await screen.findByRole("button", { name: "Favorite" });
      const dt = dataTransfer();
      fireEvent.dragStart(favorite, { dataTransfer: dt });
      fireEvent.drop(screen.getByRole("group", { name: "Toolbar preview" }), { dataTransfer: dt });
      fireEvent.dragEnd(favorite, { dataTransfer: dt });
      await act(async () => {});
      fireEvent.click(screen.getByRole("button", { name: "Save" }));
      await act(async () => {});
      // Focus survived the Save disable — on the ✕, INSIDE the dialog
      // (h07 measured active:BODY here; that killed Escape).
      const dialog = document.querySelector(".lumen-customize__surface")!;
      expect(dialog.contains(document.activeElement)).toBe(true);
      expect(document.activeElement?.getAttribute("aria-label")).toBe("Close customize panel");
      // Escape reaches the dialog node's listener → the seam fires.
      fireEvent.keyDown(document.activeElement!, { key: "Escape" });
      await act(async () => {});
      expect(heard).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener("lumen:close-panel", heard);
    }
  });

  it("the keeper pulls a dead BODY focus back onto the dialog", async () => {
    renderCustomize(new StubTransport([]));
    await screen.findByRole("dialog", { name: "Customize Player" });
    // Simulate the disabled-blur stranding focus on BODY.
    act(() => {
      (document.activeElement as HTMLElement | null)?.blur();
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 10));
    });
    const dialog = document.querySelector(".lumen-customize__surface")!;
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});

describe("the real bar's customized items (w26.i05, h07 defect 1 — state 7)", () => {
  // ControlBar.test.tsx's snapshotResult pattern: the snapshot object needs
  // the wire union's literal kinds — the double cast carries them.
  const snapshotAt = (time: number) => ({
    completed: {
      kind: "state-snapshot",
      snapshot: {
        audioDelayMs: 0,
        devices: [],
        // w26.k17 unit correction: the helper takes SECONDS (call sites
        // unchanged) and crosses into the wire's µs at the edge.
        duration: { kind: "known", time: 1_540_000_000 },
        muted: false,
        playlist: {},
        rate: 1,
        selections: {},
        state: "playing",
        subtitle: {},
        subtitleDelayMs: 0,
        time: time * 1_000_000,
        tracks: [],
        videoDelayMs: 0,
        volume: 0.5,
      },
    },
  } as unknown as CommandResponse["result"]);

  const renderItems = (transport: CommandTransport) =>
    render(
      <ClientProvider transport={transport}>
        <PlaylistProvider>
          <PlayerControlsProvider>
            <CustomizeToolbarItems />
          </PlayerControlsProvider>
        </PlaylistProvider>
      </ClientProvider>,
    );

  class SnapTransport extends StubTransport implements CommandTransport {
    constructor() {
      super([]);
    }
    override async send(request: CommandRequest): Promise<CommandResponse> {
      if (request.cmd === "system.get-state-snapshot") {
        return { gen: 1, requestId: request.requestId, result: snapshotAt(130) };
      }
      return super.send(request);
    }
  }

  it("renders the persisted band in the real bar (frame -7's payoff)", async () => {
    persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["favorite", "bookmark", "playlists", "divider", "spacer"] });
    const stub = new SnapTransport();
    renderItems(stub);
    const group = await screen.findByRole("group", { name: "Custom toolbar items" });
    // data-field-id probes (the h07 state-7 probe grammar).
    expect(group.querySelector('[data-field-id="favorite"]')).not.toBeNull();
    expect(group.querySelector('[data-field-id="bookmark"]')).not.toBeNull();
    expect(group.querySelector('[data-field-id="playlists"]')).not.toBeNull();
    // Layout primitives ride as inert separators.
    expect(group.querySelector(".lumen-toolbar-items__divider")).not.toBeNull();
    expect(group.querySelector(".lumen-toolbar-items__spacer")).not.toBeNull();
    // favorite has no surface in this shell → disabled with the honest reason.
    const favorite = screen.getByRole("button", { name: /^Favorite — / }) as HTMLButtonElement;
    expect(favorite).toBeDisabled();
    clearPersistedToolbarLayout();
  });

  it("live re-reads across Save through the lumen:toolbar-changed seam", async () => {
    const stub = new SnapTransport();
    renderItems(stub);
    // Empty band → nothing rendered.
    await act(async () => {});
    expect(screen.queryByRole("group", { name: "Custom toolbar items" })).toBeNull();
    // Save from another surface (the seam broadcast).
    act(() => {
      persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["favorite"] });
    });
    await waitFor(() =>
      expect(screen.getByRole("group", { name: "Custom toolbar items" })).toBeInTheDocument(),
    );
    clearPersistedToolbarLayout();
  });

  it("bookmark drops a media.add-bookmark at the engine-reported position", async () => {
    persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["bookmark"] });
    const stub = new SnapTransport();
    renderItems(stub);
    const bookmark = await screen.findByRole("button", { name: "Bookmark" });
    expect(bookmark).toBeEnabled();
    fireEvent.click(bookmark);
    await act(async () => {});
    const add = stub.sent.filter((request) => request.cmd === "media.add-bookmark");
    expect(add).toHaveLength(1);
    // w26.k17: the wire `time` is µs — the engine-reported 130 s position
    // crosses at the send edge (130 000 000 µs).
    expect((add[0]!.args as { time: number }).time).toBe(130_000_000);
    clearPersistedToolbarLayout();
  });

  it("playlists fires the shell's lumen:toggle-sidebar seam", async () => {
    const heard = vi.fn();
    window.addEventListener("lumen:toggle-sidebar", heard);
    try {
      persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["playlists"] });
      const stub = new SnapTransport();
      renderItems(stub);
      fireEvent.click(await screen.findByRole("button", { name: "Playlists" }));
      expect(heard).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener("lumen:toggle-sidebar", heard);
      clearPersistedToolbarLayout();
    }
  });

  it("shuffle rides the playlist context's real state (never optimistic)", async () => {
    persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["shuffle"] });
    const stub = new SnapTransport();
    renderItems(stub);
    const shuffle = await screen.findByRole("button", { name: "Shuffle" });
    fireEvent.click(shuffle);
    await act(async () => {});
    const sent = stub.sent.filter((request) => request.cmd === "playlist.set-shuffle");
    expect(sent.map((request) => (request.args as { enabled: boolean }).enabled)).toEqual([true]);
    clearPersistedToolbarLayout();
  });

  // ---- w26.k08 (j08 P2-3): the real bar's cycle tiles subscribe to engine
  // truth (completions + events) instead of re-dispatching default-based
  // args forever — the regression pins the tile actually ADVANCING.
  it("playback-speed advances through engine-confirmed completions (j08 P2-3)", async () => {
    persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["playback-speed"] });
    const stub = new StubTransport(
      [listing("playback.set-rate")],
      (request) =>
        request.cmd === "playback.set-rate"
          ? { kind: "applied-rate", rate: (request.args as { rate: number }).rate }
          : null,
    );
    renderItems(stub);
    const speed = await screen.findByRole("button", { name: "Playback speed" });
    // Default 1 → first click 1.25; the completion confirms 1.25 → next 1.5
    // (the pre-k08 behavior re-dispatched 1.25 forever).
    fireEvent.click(speed);
    await act(async () => {});
    fireEvent.click(speed);
    await act(async () => {});
    const sent = stub.sent.filter((request) => request.cmd === "playback.set-rate");
    expect(sent.map((request) => (request.args as { rate: number }).rate)).toEqual([1.25, 1.5]);
    clearPersistedToolbarLayout();
  });

  it("the real bar follows engine rate events (the box's subscription parity)", async () => {
    persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["playback-speed"] });
    const stub = new StubTransport([listing("playback.set-rate")]);
    renderItems(stub);
    const speed = await screen.findByRole("button", { name: "Playback speed" });
    // Engine truth arrives by event: rate 1.5 → the next click dispatches 2
    // (the pre-k08 bar never subscribed, so this stayed 1.25).
    act(() => {
      stub.emit({ seq: 1, kind: "playback.rate-changed", rate: 1.5 });
    });
    fireEvent.click(speed);
    await act(async () => {});
    const sent = stub.sent.filter((request) => request.cmd === "playback.set-rate");
    expect(sent.map((request) => (request.args as { rate: number }).rate)).toEqual([2]);
    clearPersistedToolbarLayout();
  });

  it("zoom advances through applied-zoom completions (j08 P2-3)", async () => {
    persistToolbarLayout({ order: CUSTOMIZE_TILE_IDS, band: ["zoom"] });
    const stub = new StubTransport(
      [listing("video.set-zoom")],
      (request) =>
        request.cmd === "video.set-zoom"
          ? { kind: "applied-zoom", factor: (request.args as { factor: number }).factor }
          : null,
    );
    renderItems(stub);
    const zoom = await screen.findByRole("button", { name: "Zoom" });
    fireEvent.click(zoom);
    await act(async () => {});
    fireEvent.click(zoom);
    await act(async () => {});
    const sent = stub.sent.filter((request) => request.cmd === "video.set-zoom");
    expect(sent.map((request) => (request.args as { factor: number }).factor)).toEqual([1.5, 2]);
    clearPersistedToolbarLayout();
  });
});
