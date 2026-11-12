// Card 4.F2 — player variants tests: variant model + persistence,
// variant-scoped layout data (design matrix), layout condition evaluation,
// keymap-driven switching, miniplayer compact transport (C2 mapping reuse),
// the W26 mini card anatomy (Miniplayer.svg) + sr-only h1 (e17 F5),
// music-mode LEFT-docked playlist column with the closable no-column state
// (w26.f11, e03 F5/F6/F8) + honest art placeholder (Player - Music.svg).

import { act, fireEvent, render, screen, within } from "@testing-library/react";
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

const renderWithClient = (ui: React.ReactElement) =>
  render(<ClientProvider transport={new MockTransport()}>{ui}</ClientProvider>);

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
    expect(screen.getByRole("complementary", { name: "Playlist" })).toBeInTheDocument();
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
    const playlist = screen.getByRole("complementary", { name: "Playlist" });
    // Player - Music-1.svg: the 377px column docks LEFT of the stage — the
    // docked-sidebar seam (not the old right-anchored emphasized panel).
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
    expect(screen.getByRole("complementary", { name: "Playlist" })).toBeInTheDocument();

    // ...the shell's toggle event closes the column — the no-column state
    // (Player - Music.png) is reachable. Async act: MusicMode mirrors the
    // dock state through a MutationObserver, whose callback lands on the
    // microtask queue after the DOM flip.
    await act(async () => {
      window.dispatchEvent(new Event(PLAYER_TOGGLE_SIDEBAR_EVENT));
    });
    expect(root).toHaveAttribute("data-sidebar", "closed");
    expect(screen.queryByRole("complementary", { name: "Playlist" })).toBeNull();
    expect(screen.getByRole("region", { name: "Album art" })).toBeInTheDocument();

    // The stage's quiet toggle re-opens it (MusicMode) — the column is
    // reachable in BOTH directions from inside music mode.
    const toggle = await screen.findByRole("button", { name: "Show queue panel" });
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    await act(async () => {
      fireEvent.click(toggle);
    });
    expect(root).toHaveAttribute("data-sidebar", "open");
    const reopened = await screen.findByRole("complementary", { name: "Playlist" });
    expect(within(reopened).getByRole("button", { name: "Open media" })).toBeInTheDocument();
    const openToggle = await screen.findByRole("button", { name: "Hide queue panel" });
    expect(openToggle).toHaveAttribute("aria-pressed", "true");
  });

  it("v2 audit — the docked music-mode playlist column carries the panel's Open-media CTA", () => {
    // The docked music-mode playlist is never a dead box: the playlist
    // panel's own empty-state CTA (card 4.D1, screens/playlist) dispatches
    // the shell-wide `lumen:open-media` event — the composition assertion
    // here pins that the docked music-mode column actually exposes it.
    renderWithClient(<PlayerScreen variant="music" />);
    const playlist = screen.getByRole("complementary", { name: "Playlist" });
    const cta = within(playlist).getByRole("button", { name: "Open media" });
    expect(cta).toBeInTheDocument();
    const heardOpenMedia = vi.fn();
    window.addEventListener("lumen:open-media", heardOpenMedia);
    fireEvent.click(cta);
    expect(heardOpenMedia).toHaveBeenCalledTimes(1);
    window.removeEventListener("lumen:open-media", heardOpenMedia);
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
