// Card 4.C1 — PlayerScreen shell tests: landmarks, empty state,
// layout-config consumption, chrome lock, keymap-driven fullscreen/ESC.
// Visual fidelity (screenshot diff) waits for 4.G1 — these are DOM-shape +
// behavior assertions per the card AC.
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import { MockTransport } from "../../client/transport";
import { PlayerScreen } from "./PlayerScreen";

/** Card 4.C2: PlayerScreen hosts the player state seam → needs a client. */
const renderWithClient = (ui: React.ReactElement) =>
  render(<ClientProvider transport={new MockTransport()}>{ui}</ClientProvider>);
import { CHROME_AUTO_HIDE_DELAY_MS } from "./useChromeAutoHide";

function setFullscreenElement(element: Element | null): void {
  Object.defineProperty(document, "fullscreenElement", {
    value: element,
    configurable: true,
  });
  document.dispatchEvent(new Event("fullscreenchange"));
}

function pressKey(key: string): void {
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keydown", { key }));
  });
}

beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
    value: vi.fn().mockResolvedValue(undefined),
    configurable: true,
  });
  Object.defineProperty(document, "exitFullscreen", {
    value: vi.fn().mockResolvedValue(undefined),
    configurable: true,
  });
  setFullscreenElement(null);
});

afterEach(() => {
  setFullscreenElement(null);
  vi.useRealTimers();
});

describe("PlayerScreen — ARIA landmarks + honest placeholders (doc 08 §4.5)", () => {
  it("renders the canvas as a labelled region, not a fake <video>/<canvas> element", () => {
    const { container } = renderWithClient(<PlayerScreen />);
    const canvas = screen.getByRole("region", { name: "Video canvas" });
    expect(canvas).toHaveAttribute("data-region-id", "video-canvas");
    expect(canvas).toHaveAttribute("data-placeholder", "engine-wiring-4.H1");
    // honest placeholder: no unwired media element is rendered
    expect(container.querySelector("video")).toBeNull();
    expect(container.querySelector("canvas")).toBeNull();
    // screen-reader note records the engine-wiring handoff (Solstice copy)
    expect(canvas.textContent).toContain("Video output connects here once the engine wires it up.");
  });

  it("renders the top bar (banner) with the screen title and the real control bar (4.C2)", () => {
    renderWithClient(<PlayerScreen />);
    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "Player" })).toBeInTheDocument();
    const controls = screen.getByRole("region", { name: "Playback controls" });
    expect(controls).toHaveAttribute("data-region-id", "controls-bar");
    // Card 4.C2: the transport cluster now lives here (play/pause + stop).
    expect(within(controls).getByRole("button", { name: /play/i })).toBeInTheDocument();
    expect(within(controls).getByRole("button", { name: "Stop" })).toBeInTheDocument();
  });

  it("renders open side panels as labelled complementary landmarks (playlist = real 4.D1 panel)", () => {
    renderWithClient(<PlayerScreen openPanels={["playlist", "media-info"]} media={{ hasVideo: true, hasChapters: false }} />);
    const playlist = screen.getByRole("complementary", { name: "Playlist" });
    expect(playlist).toHaveAttribute("data-region-id", "playlist");
    // Card 4.D1: the playlist region now renders the real panel + queue
    // semantics (empty state honest until the app shell wires playlist.load).
    expect(within(playlist).getByText("Nothing in the queue")).toBeInTheDocument();
    expect(within(playlist).getByRole("button", { name: "Shuffle" })).toBeInTheDocument();
    // media-info needs has-media (visible_when) — provided above
    expect(screen.getByRole("complementary", { name: "Media information" })).toBeInTheDocument();
    // closed panels (library) render nothing
    expect(screen.queryByRole("complementary", { name: "Library" })).toBeNull();
  });
});

describe("PlayerScreen — first-run empty state (doc 08 §5.3)", () => {
  it("shows the empty state with the Open-media affordance wired to the shell dialog event", () => {
    // Solstice redesign: the empty-state button is a REAL affordance now — it
    // dispatches the shell-wide `lumen:open-media` event (the app-shell file
    // dialog, another workstream, listens for exactly that name) instead of
    // sitting as a disabled wiring stub. The test contract follows the
    // behavior change; nothing was weakened — the click path is asserted
    // more strongly than the old disabled-stub check.
    renderWithClient(<PlayerScreen />);
    expect(screen.getByRole("heading", { level: 2, name: "No media open" })).toBeInTheDocument();
    expect(screen.getByText("Open media to start playback.")).toBeInTheDocument();
    const open = screen.getByRole("button", { name: "Open media" });
    expect(open).toBeEnabled();
    expect(open).not.toHaveAttribute("data-wiring-stub");
    const heardOpenMedia = vi.fn();
    window.addEventListener("lumen:open-media", heardOpenMedia);
    fireEvent.click(open);
    expect(heardOpenMedia).toHaveBeenCalledTimes(1);
    window.removeEventListener("lumen:open-media", heardOpenMedia);
  });

  it("hides the empty state once media is present; has-media regions appear", () => {
    renderWithClient(<PlayerScreen media={{ hasVideo: true, hasChapters: false }} />);
    expect(screen.queryByRole("heading", { name: "No media open" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Open media" })).toBeNull();
    const root = document.querySelector(".lumen-player")!;
    expect(root).toHaveAttribute("data-has-media", "true");
  });

  it("hides has-media-gated regions in the empty state (visible_when = has-media)", () => {
    renderWithClient(<PlayerScreen openPanels={["media-info"]} />);
    expect(screen.queryByRole("complementary", { name: "Media information" })).toBeNull();
  });

  it("chrome stays visible in the empty state (locked, doc 08 §5.3)", () => {
    renderWithClient(<PlayerScreen />);
    expect(document.querySelector(".lumen-player")).toHaveAttribute("data-chrome", "locked");
  });
});

describe("PlayerScreen — layout-config consumption (doc 07 §7)", () => {
  it("renders a custom layout source (regions from config, not hardcoded)", () => {
    renderWithClient(<PlayerScreen
        layoutSource={`schema = "layout.v1"
[[region]]
id = "canvas"
anchor = "center"
label_key = "player.canvas.label"
[[region]]
id = "top"
anchor = "top"
label_key = "player.region.title-bar"
`}
      />,
    );
    expect(screen.getByRole("region", { name: "Video canvas" })).toHaveAttribute("data-region-id", "canvas");
    // no controls-bar in this layout
    expect(screen.queryByRole("region", { name: "Playback controls" })).toBeNull();
  });

  it("throws loudly on an invalid layout source (never a silent fallback)", () => {
    // suppress the expected console.error from React's error boundary path
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => renderWithClient(<PlayerScreen layoutSource="schema = nope" />)).toThrow();
    spy.mockRestore();
  });
});

describe("PlayerScreen — chrome lock + auto-hide integration (doc 08 §4.3)", () => {
  it("locks chrome visible while a panel is open, even in fullscreen", () => {
    renderWithClient(<PlayerScreen openPanels={["playlist"]} />);
    const root = document.querySelector(".lumen-player")!;
    expect(root).toHaveAttribute("data-chrome", "locked");
  });

  it("hides chrome on idle in fullscreen and reveals on activity", () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    renderWithClient(<PlayerScreen media={{ hasVideo: true, hasChapters: false }} />);
    const root = document.querySelector(".lumen-player")!;
    expect(root).toHaveAttribute("data-chrome", "visible");

    act(() => {
      setFullscreenElement(root);
    });
    expect(root).toHaveAttribute("data-chrome", "visible");

    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(root).toHaveAttribute("data-chrome", "hidden");

    act(() => {
      window.dispatchEvent(new Event("pointermove"));
    });
    expect(root).toHaveAttribute("data-chrome", "visible");
  });
});

describe("PlayerScreen — keymap-driven fullscreen + ESC (doc 05 §8.1 data)", () => {
  it("F requests fullscreen on the container (fullscreen-toggle recipe from keymap data)", () => {
    renderWithClient(<PlayerScreen />);
    const root = document.querySelector(".lumen-player") as HTMLElement;
    pressKey("f");
    expect(root.requestFullscreen).toHaveBeenCalledTimes(1);
    expect(document.exitFullscreen).not.toHaveBeenCalled();
  });

  it("Escape exits while fullscreen (native events sync state honestly)", () => {
    renderWithClient(<PlayerScreen />);
    const root = document.querySelector(".lumen-player") as HTMLElement;
    act(() => {
      setFullscreenElement(root);
    });
    expect(root).toHaveAttribute("data-fullscreen", "true");
    pressKey("Escape");
    expect(document.exitFullscreen).toHaveBeenCalledTimes(1);
    // state follows the event, not the call
    act(() => {
      setFullscreenElement(null);
    });
    expect(root).toHaveAttribute("data-fullscreen", "false");
  });

  it("Escape when NOT fullscreen is forwarded to the 4.C2 seam (dialog-close half)", () => {
    const onBinding = vi.fn();
    renderWithClient(<PlayerScreen onBinding={onBinding} />);
    pressKey("Escape");
    expect(onBinding).toHaveBeenCalledTimes(1);
    expect(onBinding).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "exit-fullscreen",
        recipe: "exit-fullscreen-close-dialog",
      }),
    );
  });

  it("unbound keys forward nothing; modifier combos are not claimed by the shell", () => {
    const onBinding = vi.fn();
    renderWithClient(<PlayerScreen onBinding={onBinding} />);
    pressKey("z");
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "f", metaKey: true }));
    });
    expect(onBinding).not.toHaveBeenCalled();
    expect(document.querySelector(".lumen-player")!.requestFullscreen).not.toHaveBeenCalled();
  });

  it("I (show-controls-fullscreen) reveals hidden chrome", () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    renderWithClient(<PlayerScreen media={{ hasVideo: true, hasChapters: false }} />);
    const root = document.querySelector(".lumen-player")!;
    act(() => {
      setFullscreenElement(root);
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(root).toHaveAttribute("data-chrome", "hidden");
    pressKey("i");
    expect(root).toHaveAttribute("data-chrome", "visible");
  });
});
