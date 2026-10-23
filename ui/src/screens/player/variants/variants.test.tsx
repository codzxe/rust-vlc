// Card 4.F2 — player variants tests: variant model + persistence,
// variant-scoped layout data (design matrix), layout condition evaluation,
// keymap-driven switching, miniplayer compact transport (C2 mapping reuse),
// music-mode playlist emphasis + honest art placeholder.

import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { PlayerScreen } from "../PlayerScreen";
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

  it("music layout: art + topbar + controls + prominent playlist; no video canvas or library", () => {
    const ids = MUSIC_LAYOUT.regions.map((r) => r.id);
    expect(ids).toEqual(["album-art", "title-bar", "controls-bar", "playlist"]);
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
  it("renders the video canvas and hides panels until opened — no variant chrome", () => {
    renderWithClient(<PlayerScreen />);
    const root = screen.getByRole("region", { name: "Video canvas" }).closest(".lumen-player");
    expect(root).toHaveAttribute("data-variant", "full");
    expect(screen.queryByRole("region", { name: "Miniplayer" })).toBeNull();
    expect(screen.queryByRole("region", { name: "Album art" })).toBeNull();
    expect(screen.queryByRole("complementary", { name: "Playlist" })).toBeNull();
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
});

describe("music variant", () => {
  it("replaces the video canvas with the honest album-art placeholder (no fake art)", () => {
    renderWithClient(<PlayerScreen variant="music" />);
    const art = screen.getByRole("region", { name: "Album art" });
    expect(art).toHaveAttribute("data-region-id", "album-art");
    expect(art).toHaveAttribute("data-placeholder", "artwork-engine-data-unwired");
    expect(art.querySelector("img")).toBeNull();
    expect(art.textContent).toContain("Album art placeholder");
    expect(screen.queryByRole("region", { name: "Video canvas" })).toBeNull();
  });

  it("emphasizes the playlist panel without openPanels and hides the library", () => {
    renderWithClient(<PlayerScreen variant="music" />);
    const playlist = screen.getByRole("complementary", { name: "Playlist" });
    expect(playlist).toHaveAttribute("data-emphasized", "true");
    expect(screen.queryByRole("complementary", { name: "Library" })).toBeNull();
    // the real C2 control bar still drives transport in music mode
    const footer = document.querySelector('[data-region-id="controls-bar"]');
    expect(footer).not.toBeNull();
    expect(footer?.getAttribute("data-variant")).toBe("music");
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
