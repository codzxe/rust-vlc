// Card 4.F2 — player variants tests: variant model + persistence,
// variant-scoped layout data (design matrix), layout condition evaluation,
// keymap-driven switching, miniplayer compact transport (C2 mapping reuse),
// the W26 mini card anatomy (Miniplayer.svg) + sr-only h1 (e17 F5),
// music-mode LEFT-docked playlist column with the closable no-column state
// (w26.f11, e03 F5/F6/F8) + honest art placeholder (Player - Music.svg).

import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { PlayerScreen, PLAYER_TOGGLE_SIDEBAR_EVENT } from "../PlayerScreen";
import { DEFAULT_LAYOUT, evaluateCondition } from "../layout";
import {
  MINIPLAYER_LAYOUT,
  MUSIC_LAYOUT,
  PLAYER_VARIANT_LAYOUTS,
  VARIANT_STORAGE_KEY,
  clearPersistedVariant,
  loadPersistedVariant,
  parsePlayerVariant,
  persistVariant,
  resolveVariantLayout,
} from "./index";

import { MiniPlayer } from "./MiniPlayer";
import { PlayerControlsProvider } from "../context";
import { PlaylistProvider } from "../../playlist";
import type { PlaylistEntry } from "../../playlist";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const variantsCss = readFileSync(`${dirname}/variants.css`, "utf8");

/** Line-anchored single-rule extractor (the List.test cssRule pattern,
 * anchored so prefixed selectors cannot shadow each other). Hoisted to
 * module scope w26.k08 (j08 finding #9 — was duplicated verbatim in the
 * gfix-F and i04 describes). */
const cssRule = (selector: string): string => {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return variantsCss.match(new RegExp(`\\n${escaped}[^{]*\\{([^}]*)\\}`))?.[1] ?? "";
};

const renderWithClient = (ui: React.ReactElement, transport = new MockTransport()) => {
  const view = render(<ClientProvider transport={transport}>{ui}</ClientProvider>);
  return { ...view, transport };
};

function pressKey(key: string): void {
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keydown", { key }));
  });
}

beforeEach(() => {
  clearPersistedVariant();
  Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
    value: vi.fn().mockResolvedValue(undefined),
    configurable: true,
  });
  Object.defineProperty(document, "exitFullscreen", {
    value: vi.fn().mockResolvedValue(undefined),
    configurable: true,
  });
});

afterEach(() => {
  clearPersistedVariant();
});

describe("variant model (variants/variant.ts)", () => {
  it("parses the three known variants and rejects everything else (never guessed)", () => {
    expect(parsePlayerVariant("full")).toBe("full");
    expect(parsePlayerVariant("miniplayer")).toBe("miniplayer");
    expect(parsePlayerVariant("music")).toBe("music");
    expect(parsePlayerVariant("cinema")).toBeNull();
    expect(parsePlayerVariant(null)).toBeNull();
    expect(parsePlayerVariant(undefined)).toBeNull();
  });

  it("persists and reloads the variant through the storage seam; clear resets", () => {
    persistVariant("miniplayer");
    expect(loadPersistedVariant()).toBe("miniplayer");
    expect(globalThis.localStorage.getItem(VARIANT_STORAGE_KEY)).toBe("miniplayer");
    clearPersistedVariant();
    expect(loadPersistedVariant()).toBeNull();
  });
});

describe("variant-scoped layout data (design matrix, doc 08 §1.6)", () => {
  it("miniplayer layout declares only the compact card regions", () => {
    expect(MINIPLAYER_LAYOUT.regions.map((r) => r.id)).toEqual(["mini-thumbnail", "mini-controls"]);
    expect(MINIPLAYER_LAYOUT.regions.every((r) => r.visibleWhen === "miniplayer")).toBe(true);
  });

  it("music layout: art + controls + the LEFT-docked playlist; no topbar, canvas or library (w26.f11)", () => {
    const ids = MUSIC_LAYOUT.regions.map((r) => r.id);
    expect(ids).toEqual(["album-art", "controls-bar", "playlist"]);
    // w26.f11 (e03 F5): the playlist docks LEFT (Player - Music-1.svg) via
    // the docked-sidebar seam; e03 F8: no title-bar region (the app shell's
    // banner is the one topbar — no doubled bar).
    expect(MUSIC_LAYOUT.regions.find((r) => r.id === "playlist")?.anchor).toBe("left");
    expect(ids).not.toContain("title-bar");
    expect(ids).not.toContain("video-canvas");
    expect(ids).not.toContain("library");
  });

  it("variant layouts keep the layout.v1 schema and full maps to C1's default", () => {
    expect(PLAYER_VARIANT_LAYOUTS.full).toBe(DEFAULT_LAYOUT);
    expect(MINIPLAYER_LAYOUT.schema).toBe("layout.v1");
    expect(MUSIC_LAYOUT.schema).toBe("layout.v1");
  });

  it("a present custom layout source wins over the variant default (config owns existence)", () => {
    const custom = `schema = "layout.v1"\n\n[[region]]\nid = "only-canvas"\nanchor = "center"\nlabel_key = "player.canvas.label"\n`;
    expect(resolveVariantLayout(custom, "miniplayer").regions.map((r) => r.id)).toEqual([
      "only-canvas",
    ]);
  });

  it("variant condition tokens evaluate against the runtime variant flags", () => {
    const base = { hasMedia: true, hasVideo: true, fullscreen: false, hasChapters: false };
    expect(evaluateCondition("miniplayer", base)).toBe(false);
    expect(evaluateCondition("miniplayer", { ...base, miniplayer: true })).toBe(true);
    expect(evaluateCondition("music-mode", base)).toBe(false);
    expect(evaluateCondition("music-mode", { ...base, musicMode: true })).toBe(true);
  });
});

describe("full variant (default first run)", () => {
  it("renders the video canvas and hides tool panels until opened — no variant chrome", () => {
    // W26 Player.png composition: the docked playlist sidebar is part of
    // the full shell (pinned by PlayerScreen.test.tsx — asserted present
    // here so the variant contract stays explicit). The variant-owned
    // claim: NO miniplayer/music chrome renders, and the right-side TOOL
    // panels stay closed until a host opens them through the openPanels
    // seam (doc 07 §7) — both directions pinned below.
    const first = renderWithClient(
      <PlayerScreen media={{ hasVideo: true, hasChapters: false }} />,
    );
    const root = screen.getByRole("region", { name: "Video canvas" }).closest(".lumen-player");
    expect(root).toHaveAttribute("data-variant", "full");
    // w26.i02 (h03 #1): the player's docked sidebar is the playlists
    // BROWSER ("Playlists" — Player.png's single sidebar), not the queue
    // panel; adapted from the pre-i02 "Playlist" name.
    expect(screen.getByRole("complementary", { name: "Playlists" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Miniplayer" })).toBeNull();
    expect(screen.queryByRole("region", { name: "Album art" })).toBeNull();
    // has-media regions exist in the layout, but nothing opens until asked.
    expect(screen.queryByRole("complementary", { name: "Media information" })).toBeNull();
    expect(screen.queryByRole("complementary", { name: "Customize" })).toBeNull();
    first.unmount();

    renderWithClient(
      <PlayerScreen
        media={{ hasVideo: true, hasChapters: false }}
        openPanels={["media-info"]}
      />,
    );
    expect(screen.getByRole("complementary", { name: "Media information" })).toBeInTheDocument();
  });
});

describe("miniplayer variant", () => {
  it("renders the compact card only: thumbnail + mini transport, no top bar/panels/full bar", () => {
    renderWithClient(<PlayerScreen variant="miniplayer" />);
    const mini = screen.getByRole("region", { name: "Miniplayer" });
    expect(mini).toHaveAttribute("data-variant", "miniplayer");
    expect(within(mini).getByRole("region", { name: "Video canvas" })).toHaveAttribute(
      "data-region-id",
      "mini-thumbnail",
    );
    const bar = within(mini).getByRole("region", { name: "Miniplayer controls" });
    expect(within(bar).getByLabelText("Play")).toBeDisabled();
    expect(within(bar).getByLabelText("Expand to full player")).toBeEnabled();
    expect(within(bar).getByLabelText("Close miniplayer")).toBeEnabled();
    // compact matrix: no stop button, no volume, no track menus (Miniplayer.svg)
    expect(within(bar).queryByLabelText("Stop")).toBeNull();
    expect(within(bar).queryByLabelText("Mute")).toBeNull();
    expect(screen.queryByRole("banner")).toBeNull();
    expect(screen.queryByRole("complementary")).toBeNull();
  });

  it("w26.i04: the overlay carries the frame's centered prev · play · next row (h06 #3)", () => {
    renderWithClient(<PlayerScreen variant="miniplayer" />);
    const mini = screen.getByRole("region", { name: "Miniplayer" });
    // Miniplayer.svg Variant3: prev/play/next at video mid, close TL, expand TR.
    const prev = within(mini).getByRole("button", { name: "Previous — no media open" });
    const next = within(mini).getByRole("button", { name: "Next — no media open" });
    // Honest disabled with the reason while the queue is empty (C2 truth).
    expect(prev).toBeDisabled();
    expect(next).toBeDisabled();
    expect(prev.className).toContain("lumen-mini__skip--prev");
    expect(next.className).toContain("lumen-mini__skip--next");
    // The corners keep their frame slots; the play stays centered.
    expect(within(mini).getByRole("button", { name: "Close miniplayer" })).toBeInTheDocument();
    expect(within(mini).getByRole("button", { name: "Expand to full player" })).toBeInTheDocument();
    expect(within(mini).getByRole("button", { name: "Play" })).toBeInTheDocument();
  });

  it("reuses C2's engine-state mapping: idle ⇒ honest disabled play + disabled seek", () => {
    renderWithClient(<PlayerScreen variant="miniplayer" />);
    const bar = screen.getByRole("region", { name: "Miniplayer controls" });
    const play = within(bar).getByLabelText("Play");
    expect(play).toBeDisabled();
    expect(within(bar).getByLabelText(/Seek/)).toHaveAttribute("aria-disabled", "true");
  });

  it("expand affordance switches back to the full variant via the seam", () => {
    const onVariantChange = vi.fn();
    renderWithClient(<PlayerScreen onVariantChange={onVariantChange} />);
    pressKey("b"); // enter the miniplayer (uncontrolled persisted state)
    fireEvent.click(screen.getByLabelText("Expand to full player"));
    expect(onVariantChange).toHaveBeenLastCalledWith("full");
    expect(screen.getByRole("region", { name: "Video canvas" })).toHaveAttribute(
      "data-region-id",
      "video-canvas",
    );
  });

  it("W26 card anatomy: sr-only h1 (e17 F5), honest empty title, meta row, collapsible queue panel ABOVE the video", () => {
    renderWithClient(<PlayerScreen variant="miniplayer" />);
    // e17 F5 — the wave-a sr-only h1 contract: /miniplayer carries exactly
    // one level-1 heading, screen-reader-only (PlayerScreen's main-branch
    // h1 never renders in the miniplayer composition).
    const h1 = screen.getByRole("heading", { level: 1, name: "Miniplayer" });
    expect(h1.className).toContain("lumen-player__sr-title");
    const mini = screen.getByRole("region", { name: "Miniplayer" });
    // Honest quiet title while nothing plays — never a fabricated name.
    expect(within(mini).getByText("Nothing playing")).toBeInTheDocument();
    // Meta row: the queue toggle starts collapsed.
    const toggle = within(mini).getByRole("button", { name: "Toggle queue" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(within(mini).queryByRole("region", { name: "Queue" })).toBeNull();

    // Opening the queue renders the panel ABOVE the video area (frame
    // artboards 2/4), with the honest empty state — MockTransport exposes
    // no playlist items, so no rows are fabricated.
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    const queue = within(mini).getByRole("region", { name: "Queue" });
    const video = within(mini).getByRole("region", { name: "Video canvas" });
    expect(
      // the video FOLLOWS the queue in DOM order = panel sits above it
      Boolean(queue.compareDocumentPosition(video) & Node.DOCUMENT_POSITION_FOLLOWING),
    ).toBe(true);
    expect(within(queue).getByText("Nothing in the queue")).toBeInTheDocument();
  });

  it("w26.i04: the queue's + enqueues a picked source through playlist.enqueue (the g20-P2-3 seam)", async () => {
    const { transport } = renderWithClient(<PlayerScreen variant="miniplayer" />);
    const mini = screen.getByRole("region", { name: "Miniplayer" });
    fireEvent.click(within(mini).getByRole("button", { name: "Toggle queue" }));
    const queue = within(mini).getByRole("region", { name: "Queue" });

    // The + opens the add-to-queue popup (aria-expanded + haspopup honest).
    const add = within(queue).getByRole("button", { name: "Add to queue" });
    expect(add).toHaveAttribute("aria-haspopup", "menu");
    fireEvent.click(add);
    expect(add).toHaveAttribute("aria-expanded", "true");

    // The demo clips the open-media dialog offers — real transport-backed
    // sources, deduplicated; the ONE open-media funnel rides as the footer.
    const menu = screen.getByRole("menu", { name: "Add to queue" });
    expect(within(menu).getByRole("menuitem", { name: /Big Buck Bunny/ })).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: "Open media" })).toBeInTheDocument();

    // Picking a clip issues the REAL enqueue command through the playlist
    // context (playLater = playlist.enqueue) — the first UI caller the
    // enqueue seam has ever had (h06 #4).
    fireEvent.click(within(menu).getByRole("menuitem", { name: /Big Buck Bunny/ }));
    await waitFor(() =>
      expect(transport.sent.some((request) => request.cmd === "playlist.enqueue")).toBe(true),
    );
    const enqueued = transport.sent.find((request) => request.cmd === "playlist.enqueue");
    expect(enqueued?.args).toMatchObject({ source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } });
  });

  // w26.k08 (j08 P2-2) — the mini queue listbox keyboard path. Direct mount
  // over a seeded provider: MockTransport never echoes playlist.changed, so
  // the popup path cannot grow rows in jsdom (the enqueue path itself stays
  // pinned by the i04 test above + the shared seam's own tests, k14's tree).
  it("the queue listbox is keyboard-operable: focus, activate, Alt+arrow reorder (j08 P2-2)", async () => {
    const QUEUE_ROWS: readonly PlaylistEntry[] = [
      { id: 1, title: "Big Buck Bunny", source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" }, durationSeconds: 596 },
      { id: 2, title: "Sintel", source: { kind: "file", path: "/media/demo/sintel.mkv" }, durationSeconds: 888 },
      { id: 3, title: "Tears of Steel", source: { kind: "file", path: "/media/demo/tears-of-steel.mkv" }, durationSeconds: 734 },
    ];
    const transport = new MockTransport();
    const thumbnail = MINIPLAYER_LAYOUT.regions.find((r) => r.id === "mini-thumbnail")!;
    const controls = MINIPLAYER_LAYOUT.regions.find((r) => r.id === "mini-controls")!;
    render(
      <ClientProvider transport={transport}>
        <PlaylistProvider initialEntries={QUEUE_ROWS}>
          <PlayerControlsProvider>
            <MiniPlayer thumbnailRegion={thumbnail} controlsRegion={controls} onExpand={() => {}} />
          </PlayerControlsProvider>
        </PlaylistProvider>
      </ClientProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Toggle queue" }));
    const listbox = screen.getByRole("listbox", { name: "Queue" });
    // The container owns focus (VirtualList's viewport tabIndex grammar).
    expect(listbox).toHaveAttribute("tabindex", "0");
    expect(within(listbox).getAllByRole("option")).toHaveLength(3);
    // No current entry → the first row is the active descendant.
    expect(listbox).toHaveAttribute("aria-activedescendant", "lumen-mini-row-0");

    // Arrows roam (clamped at the edges); Home/End jump.
    fireEvent.keyDown(listbox, { key: "ArrowDown" });
    expect(listbox).toHaveAttribute("aria-activedescendant", "lumen-mini-row-1");
    fireEvent.keyDown(listbox, { key: "End" });
    expect(listbox).toHaveAttribute("aria-activedescendant", "lumen-mini-row-2");
    fireEvent.keyDown(listbox, { key: "ArrowDown" }); // clamped — no wrap
    expect(listbox).toHaveAttribute("aria-activedescendant", "lumen-mini-row-2");
    fireEvent.keyDown(listbox, { key: "Home" });
    expect(listbox).toHaveAttribute("aria-activedescendant", "lumen-mini-row-0");

    // Enter/Space activate the active row — the real play-item command.
    fireEvent.keyDown(listbox, { key: "ArrowDown" });
    fireEvent.keyDown(listbox, { key: "Enter" });
    fireEvent.keyDown(listbox, { key: "ArrowDown" });
    fireEvent.keyDown(listbox, { key: " " });
    await act(async () => {});
    const played = transport.sent.filter((r) => r.cmd === "playlist.play-item");
    expect(played.map((r) => (r.args as { id: number }).id)).toEqual([2, 3]);

    // Alt+Up/Down reorder the focused row through playlist.move — the same
    // command + dragToMoveIndex semantics the mouse DnD path issues.
    fireEvent.keyDown(listbox, { key: "ArrowUp", altKey: true }); // row 3 → slot 1
    await act(async () => {});
    const moved = transport.sent.filter((r) => r.cmd === "playlist.move");
    expect(moved.map((r) => r.args)).toEqual([{ id: 3, toIndex: 1 }]);
    // The focus index stays on the slot the row moved TO (index 1).
    expect(listbox).toHaveAttribute("aria-activedescendant", "lumen-mini-row-1");
    // NO-ECHO CAVEAT (honest jsdom truth): MockTransport never confirms the
    // move, so the DOM keeps the stale order — the second Alt+Down moves the
    // row NOW sitting at the focused index (Sintel). Against an echoing
    // transport (the demo/live engine) the DOM reorders first and the same
    // sequence moves the SAME item (3→1 then 3→2) — that live half rides the
    // provider's `moved` handling + the dragToMoveIndex unit pins.
    fireEvent.keyDown(listbox, { key: "ArrowDown", altKey: true }); // back down one
    await act(async () => {});
    expect(transport.sent.filter((r) => r.cmd === "playlist.move").map((r) => r.args)).toEqual([
      { id: 3, toIndex: 1 },
      { id: 2, toIndex: 2 },
    ]);
    // Edge no-ops: Alt+Down at the last row never re-dispatches.
    fireEvent.keyDown(listbox, { key: "ArrowDown", altKey: true });
    await act(async () => {});
    expect(transport.sent.filter((r) => r.cmd === "playlist.move")).toHaveLength(2);
  });
});

describe("music variant", () => {
  it("replaces the video canvas with the honest album-art placeholder (no fake art, no stage waveform)", () => {
    renderWithClient(<PlayerScreen variant="music" />);
    const art = screen.getByRole("region", { name: "Album art" });
    expect(art).toHaveAttribute("data-region-id", "album-art");
    expect(art).toHaveAttribute("data-placeholder", "artwork-engine-data-unwired");
    expect(art.querySelector("img")).toBeNull();
    expect(art.textContent).toContain("Album art appears once the engine reports it.");
    expect(screen.queryByRole("region", { name: "Video canvas" })).toBeNull();
    // w26.f11 (e03 F7): no doubled seek surface in the stage — the waveform
    // strip is gone; the frame's tick field lives INSIDE the real C2 seek
    // cluster in the controls row (one interactive seek surface only).
    expect(art.querySelector(".lumen-music__waveform")).toBeNull();
    const footer = document.querySelector('[data-region-id="controls-bar"]');
    expect(footer?.querySelector(".lumen-seekbar")).not.toBeNull();
  });

  it("docks the playlist column LEFT without openPanels and hides the library (w26.f11, e03 F5)", () => {
    renderWithClient(<PlayerScreen variant="music" />);
    const root = document.querySelector(".lumen-player");
    expect(root).toHaveAttribute("data-sidebar", "open");
    // w26.i02 (h03 #1): the docked column is the playlists BROWSER —
    // Player - Music-1.svg's 377px column rendered through the same
    // docked-sidebar seam (not the old right-anchored emphasized panel).
    const playlist = screen.getByRole("complementary", { name: "Playlists" });
    expect(playlist).toHaveAttribute("data-region-id", "playlist");
    expect(playlist.className).toContain("lumen-player__sidebar");
    expect(playlist.getAttribute("data-emphasized")).toBeNull();
    const stage = screen.getByRole("region", { name: "Album art" });
    expect(
      // the stage FOLLOWS the dock in DOM order = the column sits left of it
      Boolean(playlist.compareDocumentPosition(stage) & Node.DOCUMENT_POSITION_FOLLOWING),
    ).toBe(true);
    expect(screen.queryByRole("complementary", { name: "Library" })).toBeNull();
    // the real C2 control bar still drives transport in music mode
    const footer = document.querySelector('[data-region-id="controls-bar"]');
    expect(footer).not.toBeNull();
    expect(footer?.getAttribute("data-variant")).toBe("music");
  });

  it("e03 F6: closing the playlist collapses to the Player_-_Music.png no-column state; the stage toggle re-opens it", async () => {
    renderWithClient(<PlayerScreen variant="music" />);
    const root = document.querySelector(".lumen-player");
    // Open by default (Player - Music-1.svg composition)...
    expect(screen.getByRole("complementary", { name: "Playlists" })).toBeInTheDocument();

    // ...the shell's toggle event closes the column — the no-column state
    // (Player - Music.png) is reachable. Async act: MusicMode mirrors the
    // dock state through a MutationObserver, whose callback lands on the
    // microtask queue after the DOM flip.
    await act(async () => {
      window.dispatchEvent(new Event(PLAYER_TOGGLE_SIDEBAR_EVENT));
    });
    expect(root).toHaveAttribute("data-sidebar", "closed");
    expect(screen.queryByRole("complementary", { name: "Playlists" })).toBeNull();
    expect(screen.getByRole("region", { name: "Album art" })).toBeInTheDocument();

    // The stage's quiet toggle re-opens it (MusicMode) — the column is
    // reachable in BOTH directions from inside music mode. w26.i02: the
    // dock is the playlists browser, so the toggle says playlists.
    const toggle = await screen.findByRole("button", { name: "Show playlists panel" });
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    await act(async () => {
      fireEvent.click(toggle);
    });
    expect(root).toHaveAttribute("data-sidebar", "open");
    const reopened = await screen.findByRole("complementary", { name: "Playlists" });
    expect(within(reopened).getByRole("button", { name: "Create playlist" })).toBeInTheDocument();
    const openToggle = await screen.findByRole("button", { name: "Hide playlists panel" });
    expect(openToggle).toHaveAttribute("aria-pressed", "true");
  });

  it("v2 audit — the docked music-mode column is the live playlists browser (w26.i02 dock, real store rows)", () => {
    // The docked music-mode column is never a dead box: since w26.i02
    // (h03 #1) it is the shell's playlists browser over the REAL
    // saved-playlists store (the same seam as /playlist and the shell
    // sidebar — seeds "Cool Playlist"/"Cool Songs"/"Anime to Watch").
    renderWithClient(<PlayerScreen variant="music" />);
    const playlist = screen.getByRole("complementary", { name: "Playlists" });
    expect(within(playlist).getByRole("button", { name: "Create playlist" })).toBeInTheDocument();
    expect(within(playlist).getByRole("button", { name: "Cool Playlist" })).toBeInTheDocument();
  });
});

describe("keymap-driven switching (variantKeymap seam)", () => {
  it("B toggles full ⇄ miniplayer", () => {
    const { container } = renderWithClient(<PlayerScreen />);
    pressKey("b");
    expect(container.querySelector(".lumen-player")).toHaveAttribute("data-variant", "miniplayer");
    pressKey("b");
    expect(container.querySelector(".lumen-player")).toHaveAttribute("data-variant", "full");
  });

  it("G toggles full ⇄ music mode", () => {
    const { container } = renderWithClient(<PlayerScreen />);
    pressKey("g");
    expect(container.querySelector(".lumen-player")).toHaveAttribute("data-variant", "music");
    pressKey("g");
    expect(container.querySelector(".lumen-player")).toHaveAttribute("data-variant", "full");
  });

  it("reports every switch through onVariantChange", () => {
    const onVariantChange = vi.fn();
    renderWithClient(<PlayerScreen onVariantChange={onVariantChange} />);
    pressKey("b");
    pressKey("g");
    expect(onVariantChange).toHaveBeenNthCalledWith(1, "miniplayer");
    expect(onVariantChange).toHaveBeenNthCalledWith(2, "music");
  });

  it("persists the variant across mounts (state survives remount)", () => {
    const first = renderWithClient(<PlayerScreen />);
    pressKey("g");
    first.unmount();
    const second = renderWithClient(<PlayerScreen />);
    expect(second.container.querySelector(".lumen-player")).toHaveAttribute(
      "data-variant",
      "music",
    );
  });
});

// ------------------------------------------------------- w26.gfix-F pins ---
// The frame-pinned geometry this lane re-anchored (issue #62): CSS-rule
// guards so the next refactor cannot silently drop a pin back to the
// pre-fix state (the strip-portal dead-selector class of regression —
// measured live in /tmp/cap-gfixF, w26.gfix-F log).
describe("w26.gfix-F frame pins (variants.css)", () => {
  it("music transport is the frame's frosted band, not the opaque chrome strip (g06 P2)", () => {
    // Player - Music.svg Controls group: 1920×54 #191919@0.95 + 1px #222222
    // hairline + 50px background blur (the g06 finding's "no band" reading
    // was wrong — the SVG pins all three; the fix killed the OPAQUE #161616).
    const band = cssRule('.app-shell__strip > .lumen-player__controls[data-variant="music"]');
    expect(band).toContain("color-mix(in srgb, var(--color-surface-panel) 95%, transparent)");
    expect(band).toContain("border-top: 1px solid var(--color-surface-raised)");
    expect(band).toContain("backdrop-filter: blur(50px)");
  });

  it("the tick field anchors on the footer's own data-variant (alive under the strip portal)", () => {
    // The Lead's AppShellStripContext portal hoists the band out from under
    // .lumen-player[data-variant] — the old anchor was dead CSS in the app.
    const track = cssRule(
      '.lumen-player__controls[data-variant="music"] .lumen-controls__seek .lumen-slider__track',
    );
    expect(track).toContain("repeating-linear-gradient");
    const thumb = cssRule(
      '.lumen-player__controls[data-variant="music"] .lumen-controls__seek .lumen-slider__thumb',
    );
    expect(thumb).toContain("width: 4px");
    expect(thumb).toContain("border-radius: 2px");
  });

  it("mini queue: 44px row pitch, body/head band inversion, quiet title (g07 P3-3/P3-4)", () => {
    // Miniplayer.svg queue: rows 591/635/679/723 (44px), body #1B1B1B with a
    // #151515 header band, "Queue" #8F8F8F.
    expect(cssRule(".lumen-mini__queue .lumen-lrow--compact")).toContain("height: 44px");
    const panel = variantsCss.match(/\.lumen-mini__queue \{([^}]*)\}/)?.[1] ?? "";
    expect(panel).toContain("var(--color-surface-panel)");
    expect(cssRule(".lumen-mini__queue-head")).toContain("var(--color-surface-muted)");
    expect(
      variantsCss.match(/\.lumen-mini__queue-title \{([^}]*)\}/)?.[1] ?? "",
    ).toContain("var(--color-content-secondary)");
  });

  it("icon wrappers round rx4 (g07 P3-1); the play pill keeps its functional circle", () => {
    // Icon Wrapper.svg hover chip rx4 — scoped to the compact card's hits so
    // the shared control radius stays for other IconButtons.
    expect(cssRule(".lumen-mini .lumen-icon-button")).toContain("var(--radius-inset)");
    expect(cssRule(".lumen-mini__controls .lumen-mini__play")).toContain("border-radius: 50%");
  });

  it("Music-1 window-centers the artwork while the dock is open (g06 P1-3 residual)", () => {
    // (rail 52 + dock 300) / 2 = 176px slide to the window midpoint, as a
    // relative left (a negative margin only shifts the center half as far —
    // flex centers the MARGIN box), clamped so the art never slides under
    // the open dock on narrow windows. w26.i04: the clamp constant is
    // re-pinned for the proportional art (553.5px): clamp edge = dock right
    // 352 + art/2 276.75 = 628.75; 628.75 − 176 = 452.75 (was 436 @520 art).
    const stage = cssRule(
      '.lumen-player[data-variant="music"][data-sidebar="open"] .lumen-music__stage',
    );
    expect(stage).toContain("position: relative");
    expect(stage).toContain("left: max(-176px, calc(452.75px - 50vw))");
  });
});

// ------------------------------------------------------- w26.i04 pins ---
// The h06 fix-cycle frame pins: the mini card's vertical rhythm (h06 #2),
// the overlay skip row (h06 #3), the music transport-tail reduction (h06
// #1) and the miniplayer shell-sidebar seam (h06 #5). CSS-rule guards so
// the next refactor cannot silently drop a pin back to the pre-fix state.
describe("w26.i04 frame pins (variants.css)", () => {
  it("mini card vertical rhythm: 26.4px seek→title gap + the 339px card bottom inset (h06 #2)", () => {
    expect(cssRule(".lumen-mini__title")).toContain("margin: 20.4px 0 0");
    // Line-anchored on the bare card selector (a cssRule( query would match
    // the earlier .lumen-mini .lumen-icon-button wrapper rule first).
    const card = variantsCss.match(/\n\.lumen-mini \{([^}]*)\}/)?.[1] ?? "";
    expect(card).toContain("padding: 8px 8px 14.4px");
    // The seek bleeds 1px past the video each side (frame 375 vs 373). The
    // explicit width is REQUIRED: slider.css's `.lumen-seekbar { width: 100% }`
    // is a definite width, so the −1px margins alone only SHIFT the 373px
    // well left (measured /tmp/cap-w26i04) — calc(100% + 2px) restores the
    // frame's 375px well with the bleed on both edges.
    const seek = cssRule(".lumen-mini__controls .lumen-mini__seek");
    expect(seek).toContain("margin-inline: -1px");
    expect(seek).toContain("width: calc(100% + 2px)");
  });

  it("overlay skip row: prev/next centers ±75px of the play center at video mid (h06 #3)", () => {
    expect(cssRule(".lumen-mini__transport .lumen-mini__skip")).toContain("top: 50%");
    expect(cssRule(".lumen-mini__transport .lumen-mini__skip--prev")).toContain(
      "left: calc(50% - 75px)",
    );
    expect(cssRule(".lumen-mini__transport .lumen-mini__skip--next")).toContain(
      "left: calc(50% + 75px)",
    );
  });

  it("music transport tail reduces to the frame's clusters; the well takes its 65% share (h06 #1)", () => {
    // Player - Music.svg Controls ends with volume + the far-right menu —
    // the audio/subtitles/settings/queue triggers drop out under the music
    // variant (a REAL reduction: display:none leaves the a11y tree).
    for (const control of ["audio", "subtitles", "settings", "queue"]) {
      const body = variantsCss.match(
        new RegExp(`\\.lumen-player__controls\\[data-variant="music"\\] \\[data-control="${control}"\\][^{]*\\{([^}]*)\\}`),
      )?.[1];
      expect(body, `music variant must pin [data-control=${control}]`).toContain("display: none");
    }
  });

  it("miniplayer variant suppresses the shell playlists sidebar (h06 #5, the :has() composition seam)", () => {
    expect(variantsCss).toContain(
      '.app-shell__frame:has(.lumen-player[data-variant="miniplayer"]) > .app-shell__sidebar',
    );
  });

  it("music art claims the frame's proportional share: 61.5vmin ≈ 553.5px at 1440×900 (h06)", () => {
    expect(cssRule(".lumen-music__artwork")).toContain("width: min(61.5vmin, 554px)");
    expect(cssRule(".lumen-music__stage")).toContain("max-width: 554px");
  });
});
