// PlayerScreen shell tests — the W26 Player.png composition: docked
// playlist sidebar, canvas region + honest placeholders, control-bar slot,
// layout-config consumption, chrome lock, keymap-driven fullscreen/ESC,
// sidebar-toggle seam, now-playing identity. DOM-shape + behavior
// assertions per the card ACs (visual fidelity: dev-shots + VLM loop).
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import { MockTransport } from "../../client/transport";
import type { CommandTransport, CommandResponse, CommandRequest, Event } from "../../client/transport";
import { fullMediaInfo } from "../media-info/fixtures";
import { OPEN_MEDIA_EVENT } from "../open-media";
import { PlayerScreen, PLAYER_TOGGLE_SIDEBAR_EVENT } from "./PlayerScreen";

/** The DOM Event (the catalog `Event` type is imported as a spy shape). */
type DomEvent = globalThis.Event;

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
    // screen-reader note records the engine-wiring handoff
    expect(canvas.textContent).toContain("Video appears here during playback");
  });

  it("composes the docked playlist sidebar (Player.png) with the real 4.D1 panel", () => {
    // Cross-lane pin: the dock is part of the player's default composition
    // (open first-run — variants.test.tsx asserts it present in the full
    // variant; see the PlayerScreen.tsx composition note for the open
    // Player.png single-sidebar question).
    renderWithClient(<PlayerScreen />);
    const playlist = screen.getByRole("complementary", { name: "Playlist" });
    expect(playlist).toHaveAttribute("data-region-id", "playlist");
    expect(playlist).toHaveClass("lumen-player__sidebar");
    // Card 4.D1: the docked panel renders the real queue semantics (empty
    // state honest until the app shell wires playlist.load).
    expect(within(playlist).getByText("Nothing in the queue")).toBeInTheDocument();
    expect(within(playlist).getByRole("button", { name: "Shuffle" })).toBeInTheDocument();
  });

  it("renders the bottom control bar slot with the real control bar (4.C2)", () => {
    renderWithClient(<PlayerScreen />);
    const controls = screen.getByRole("region", { name: "Playback controls" });
    expect(controls).toHaveAttribute("data-region-id", "controls-bar");
    // Card 4.C2: the transport cluster now lives here (play/pause + stop).
    expect(within(controls).getByRole("button", { name: /play/i })).toBeInTheDocument();
    expect(within(controls).getByRole("button", { name: "Stop" })).toBeInTheDocument();
  });

  it("does not duplicate the window title bar (the app shell owns it, W26)", () => {
    renderWithClient(<PlayerScreen />);
    // The default W26 layout declares no top region: no player top strip, no
    // VISIBLE screen title (the window title bar is the app shell's banner).
    expect(document.querySelector(".lumen-player__topbar")).toBeNull();
    // The level-1 heading exists but ONLY as a screen-reader-only element
    // (a11y + the App first-run-gate tests rely on it) — never visible.
    const srTitle = screen.getByRole("heading", { level: 1, name: "Player" });
    expect(srTitle.className).toContain("lumen-player__sr-title");
    // No banner the PLAYER renders. The only <header> elements on screen are
    // the sibling panels' internal headers scoped inside complementary
    // landmarks — generic (NOT banner) in real accessibility trees per
    // HTML-AAM; jsdom's role engine lacks that scoping exception, so filter
    // by scope. Any banner OUTSIDE a complementary region would be a
    // title-bar duplicate rendered by this screen — there must be none.
    const unscopedBanner = screen
      .queryAllByRole("banner")
      .find((el) => el.closest('[role="complementary"]') === null);
    expect(unscopedBanner).toBeUndefined();
  });

  it("renders open tool panels as labelled complementary landmarks; closed ones render nothing", () => {
    renderWithClient(
      <PlayerScreen
        openPanels={["media-info"]}
        media={{ hasVideo: true, hasChapters: false }}
      />,
    );
    // media-info needs has-media (visible_when) — provided above
    expect(screen.getByRole("complementary", { name: "Media information" })).toBeInTheDocument();
    // The W26 default layout declares no library region — nothing renders.
    expect(screen.queryByRole("complementary", { name: "Library" })).toBeNull();
  });
});

describe("PlayerScreen — docked sidebar toggle seam (W26, lumen:toggle-sidebar)", () => {
  it("is open first-run and collapses/expands on the shell's toggle event", () => {
    renderWithClient(<PlayerScreen />);
    const root = document.querySelector(".lumen-player")!;
    expect(root).toHaveAttribute("data-sidebar", "open");
    expect(screen.getByRole("complementary", { name: "Playlist" })).toBeInTheDocument();

    act(() => {
      window.dispatchEvent(new Event(PLAYER_TOGGLE_SIDEBAR_EVENT));
    });
    expect(root).toHaveAttribute("data-sidebar", "closed");
    expect(screen.queryByRole("complementary", { name: "Playlist" })).toBeNull();

    act(() => {
      window.dispatchEvent(new Event(PLAYER_TOGGLE_SIDEBAR_EVENT));
    });
    expect(root).toHaveAttribute("data-sidebar", "open");
    expect(screen.getByRole("complementary", { name: "Playlist" })).toBeInTheDocument();
  });

  it("openPanels forces the sidebar visible — a host-requested panel never hides", () => {
    renderWithClient(<PlayerScreen openPanels={["playlist"]} />);
    act(() => {
      window.dispatchEvent(new Event(PLAYER_TOGGLE_SIDEBAR_EVENT));
    });
    expect(screen.getByRole("complementary", { name: "Playlist" })).toBeInTheDocument();
  });
});

describe("PlayerScreen — now-playing identity on the canvas (Player.png artwork stand-in)", () => {
  /** Transport that answers like an idle mock but can EMIT engine events —
   * the honest path for the canvas identity (engine truth only). */
  function emittingTransport(): CommandTransport & { emit: (event: Event) => void } {
    const listeners = new Set<(event: Event) => void>();
    const wire: CommandTransport = {
      subscribe(listener) {
        listeners.add(listener);
        return () => {
          listeners.delete(listener);
        };
      },
      async send(request: CommandRequest): Promise<CommandResponse> {
        return {
          gen: 1,
          requestId: request.requestId,
          result: {
            error: {
              code: "feature-not-exposed",
              details: { cmd: request.cmd },
              message: "mock transport: command not simulated yet",
              stage: "control",
            },
          },
        };
      },
    };
    return {
      ...wire,
      emit: (event: Event) => {
        for (const listener of listeners) listener(event);
      },
    };
  }

  it("centers the engine-reported title while a session is playing (no overlay)", () => {
    const transport = emittingTransport();
    render(<ClientProvider transport={transport}><PlayerScreen /></ClientProvider>);
    // Engine truth: a playing session + the info burst (title, video track).
    act(() => {
      transport.emit({
        seq: 1,
        kind: "session.state-changed",
        from: "idle",
        to: "opening",
      });
      transport.emit({ seq: 2, kind: "session.state-changed", from: "opening", to: "playing" });
      transport.emit({ seq: 3, kind: "media.info-ready", info: fullMediaInfo });
      transport.emit({ seq: 4, kind: "tracks.list-ready", tracks: fullMediaInfo.tracks });
    });
    const now = document.querySelector(".lumen-player__now")!;
    expect(now).toHaveAttribute("data-media-kind", "video");
    expect(screen.getByRole("heading", { name: "Big Buck Bunny" })).toBeInTheDocument();
    // the first-run hero is gone once media is up
    expect(screen.queryByRole("heading", { name: "No media open" })).toBeNull();
  });

  it("renders nothing over the canvas when the engine stays idle (honesty)", () => {
    renderWithClient(<PlayerScreen />);
    expect(document.querySelector(".lumen-player__now")).toBeNull();
  });
});

describe("PlayerScreen — first-run empty state (doc 08 §5.3)", () => {
  it("shows the empty state with the Open-media affordance wired to the shell dialog event", () => {
    // The empty-state button is a REAL affordance — it dispatches the
    // shell-wide `lumen:open-media` event (the app-shell file dialog,
    // another workstream, listens for exactly that name). The test contract
    // follows the behavior; the click path is asserted. Queries scope to the
    // CANVAS region — the hero's home — because the docked queue panel
    // (playlist lane) renders its own honest empty-queue CTA with the same
    // label; scoping keeps this assertion about THIS surface.
    renderWithClient(<PlayerScreen />);
    const canvas = screen.getByRole("region", { name: "Video canvas" });
    expect(
      within(canvas).getByRole("heading", { level: 2, name: "No media open" }),
    ).toBeInTheDocument();
    expect(within(canvas).getByText("Open media to start playback.")).toBeInTheDocument();
    const open = within(canvas).getByRole("button", { name: "Open media" });
    expect(open).toBeEnabled();
    expect(open).not.toHaveAttribute("data-wiring-stub");
    const heardOpenMedia = vi.fn();
    window.addEventListener("lumen:open-media", heardOpenMedia);
    fireEvent.click(open);
    expect(heardOpenMedia).toHaveBeenCalledTimes(1);
    window.removeEventListener("lumen:open-media", heardOpenMedia);
  });

  it("hides the empty state once media is present; has-media regions appear", () => {
    renderWithClient(
      <PlayerScreen
        media={{ hasVideo: true, hasChapters: false }}
        openPanels={["media-info"]}
      />,
    );
    const canvas = screen.getByRole("region", { name: "Video canvas" });
    expect(within(canvas).queryByRole("heading", { name: "No media open" })).toBeNull();
    // The CANVAS hero + its CTA are gone. The docked queue panel's own
    // empty-queue CTA (playlist lane, same label) stays honest to ITS data —
    // the queue is still empty — so scope the absence to the canvas region.
    expect(within(canvas).queryByRole("button", { name: "Open media" })).toBeNull();
    const root = document.querySelector(".lumen-player")!;
    expect(root).toHaveAttribute("data-has-media", "true");
    // The empty state's has-media gate lifts: the visible_when = "has-media"
    // media-info tool panel now renders for a host that opened it (it stayed
    // hidden in the empty state — see the test below).
    expect(
      screen.getByRole("complementary", { name: "Media information" }),
    ).toBeInTheDocument();
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
    // A layout that declares a top region gets the player top strip with the
    // screen title (the default W26 layout does not — the shell owns the
    // window title bar).
    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "Player" })).toBeInTheDocument();
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

describe("PlayerScreen — Go-to-Time dialog wiring (issue #32, keymaps/default.toml jump-to-time)", () => {
  const pressCtrlT = (): void => {
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "t", ctrlKey: true }),
      );
    });
  };

  it("Ctrl+T with media opens the Go-to-Time dialog (prefilled position)", () => {
    renderWithClient(<PlayerScreen media={{ hasVideo: false, hasChapters: false }} />);
    expect(screen.queryByRole("dialog")).toBeNull();
    pressCtrlT();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Go to time" })).toBeInTheDocument();
    // Prefill is the engine-reported position (0 → "0:00", VLC parity).
    expect(screen.getByLabelText("Time")).toHaveValue("0:00");
  });

  it("the when = \"has-media\" gate is honest: an idle session ignores Ctrl+T", () => {
    renderWithClient(<PlayerScreen />);
    pressCtrlT();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Ctrl+Shift+T is NOT the combo — no dialog (exact-modifier matching)", () => {
    renderWithClient(<PlayerScreen media={{ hasVideo: false, hasChapters: false }} />);
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "t", ctrlKey: true, shiftKey: true }),
      );
    });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Escape in the open dialog closes it (the Dialog focus trap owns the key)", () => {
    renderWithClient(<PlayerScreen media={{ hasVideo: false, hasChapters: false }} />);
    pressCtrlT();
    const dialog = screen.getByRole("dialog");
    act(() => {
      fireEvent.keyDown(dialog, { key: "Escape" });
    });
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

describe("PlayerScreen — canvas context menu (w26.c6, right-click menu.svg)", () => {
  /** Right-click the canvas region; returns the opened menu. */
  const openCanvasMenu = async (): Promise<HTMLElement> => {
    const canvas = screen.getByRole("region", { name: "Video canvas" });
    fireEvent.contextMenu(canvas, { clientX: 300, clientY: 200 });
    return await screen.findByRole("menu", { name: "Player actions" });
  };

  /** Interactive menu rows in DOM order (item + checkbox/radio faces). */
  const menuItems = (menu: HTMLElement): HTMLElement[] =>
    [...menu.querySelectorAll<HTMLElement>('[role="menuitem"], [role="menuitemcheckbox"], [role="menuitemradio"]')];

  it("opens the frame's 14 rows in pinned order with 3 separators", async () => {
    renderWithClient(<PlayerScreen />);
    const menu = await openCanvasMenu();
    // REAL frame labels (right-click menu.svg), pinned order.
    expect([...menu.querySelectorAll<HTMLElement>(".lumen-menu__label")].map((label) => label.textContent)).toEqual([
      "Play",
      "Stop",
      "Previous Item",
      "Next Item",
      "Record",
      "Hide UI",
      "Playback",
      "Video",
      "Audio",
      "Subtitle",
      "More Settings",
      "Playlist",
      "Open Media",
      "Quit VLC",
    ]);
    expect(within(menu).getAllByRole("separator")).toHaveLength(3);
    // WAI-ARIA: opening a menu moves focus onto the first item.
    expect(menuItems(menu)[0]).toHaveFocus();
  });

  it("accelerators are display strings from keymap DATA (frame spelling)", async () => {
    renderWithClient(<PlayerScreen />);
    const menu = await openCanvasMenu();
    // HOTKEY_BINDINGS rows: navigation.toggle-play-pause=Space, navigation.stop=S,
    // navigation.prev-track=P, navigation.next-track=N, video.record=Shift+R
    // (the keymap's D-1 resolution of the frame's "R"), window.n=Ctrl+\,
    // media.quit=Ctrl+Q — re-joined in the frame's "Ctrl + …" spelling.
    expect([...menu.querySelectorAll<HTMLElement>(".lumen-menu__accelerator")].map((acc) => acc.textContent)).toEqual([
      "Space",
      "S",
      "P",
      "N",
      "Shift + R",
      "Ctrl + \\",
      "Ctrl + Q",
    ]);
  });

  it("honest disabled rows: transport needs media, queue rows need a queue, no-seam rows stay inert", async () => {
    renderWithClient(<PlayerScreen />);
    const menu = await openCanvasMenu();
    const disabled = ["Play", "Stop", "Previous Item", "Next Item", "Record", "Hide UI", "Quit VLC"];
    for (const label of disabled) {
      const item = menuItems(menu).find((row) => row.textContent?.startsWith(label))!;
      expect(item, label).toHaveAttribute("aria-disabled", "true");
    }
    // The seven submenu parents stay active (they open honest content).
    for (const label of ["Playback", "Video", "Audio", "Subtitle", "More Settings", "Playlist", "Open Media"]) {
      const item = menuItems(menu).find((row) => row.textContent?.startsWith(label))!;
      expect(item, label).not.toHaveAttribute("aria-disabled");
    }
  });

  it("Playback submenu: real shuffle + repeat commands through the playlist seam", async () => {
    const transport = new MockTransport();
    render(
      <ClientProvider transport={transport}>
        <PlayerScreen />
      </ClientProvider>,
    );
    const menu = await openCanvasMenu();
    fireEvent.click(within(menu).getByText("Playback"));
    const submenu = await screen.findByRole("menu", { name: "Playback" });
    expect([...submenu.querySelectorAll<HTMLElement>(".lumen-menu__label")].map((label) => label.textContent)).toEqual([
      "Shuffle",
      "Repeat: off",
      "Repeat: all",
      "Repeat: one",
    ]);
    fireEvent.click(within(submenu).getByText("Shuffle"));
    const req = transport.sent.find((r) => r.cmd === "playlist.set-shuffle");
    expect(req && "args" in req ? req.args : null).toEqual({ enabled: true });
  });

  it("Video/Audio/Subtitle submenus open onto the honest no-media row (never a fabricated list)", async () => {
    renderWithClient(<PlayerScreen />);
    const menu = await openCanvasMenu();
    for (const label of ["Video", "Audio", "Subtitle"]) {
      fireEvent.click(within(menu).getByText(label));
      const submenu = await screen.findByRole("menu", { name: label });
      const rows = [...submenu.querySelectorAll<HTMLElement>('[role="menuitem"]')];
      expect(rows).toHaveLength(1);
      expect(rows[0]!.textContent).toBe("no media open");
      expect(rows[0]).toHaveAttribute("aria-disabled", "true");
      // Escape walks back out of the submenu before the next kind.
      fireEvent.keyDown(rows[0]!, { key: "Escape" });
    }
  });

  it("Playlist submenu toggles the docked queue panel (this screen's real seam)", async () => {
    renderWithClient(<PlayerScreen />);
    const player = document.querySelector<HTMLElement>("[data-screen='player']")!;
    expect(player).toHaveAttribute("data-sidebar", "open");
    const menu = await openCanvasMenu();
    fireEvent.click(within(menu).getByText("Playlist"));
    const submenu = await screen.findByRole("menu", { name: "Playlist" });
    fireEvent.click(within(submenu).getByText("Hide queue panel"));
    expect(document.querySelector("[data-screen='player']")!).toHaveAttribute("data-sidebar", "closed");
  });

  it("Open Media submenu: the shell event + the network-stream route", async () => {
    const seen: string[] = [];
    const onEvent = (event: DomEvent): void => {
      seen.push(event.type);
    };
    window.addEventListener(OPEN_MEDIA_EVENT, onEvent);
    const transport = new MockTransport();
    try {
      render(
        <MemoryRouter initialEntries={["/player"]}>
          <Routes>
            <Route
              path="/player"
              element={
                <ClientProvider transport={transport}>
                  <PlayerScreen />
                </ClientProvider>
              }
            />
            <Route path="/open-network-stream" element={<div>network-stream-screen</div>} />
          </Routes>
        </MemoryRouter>,
      );
      const menu = await openCanvasMenu();
      fireEvent.click(within(menu).getByText("Open Media"));
      const submenu = await screen.findByRole("menu", { name: "Open Media" });
      fireEvent.click(within(submenu).getByText("Open media…"));
      expect(seen).toEqual([OPEN_MEDIA_EVENT]);
    } finally {
      window.removeEventListener(OPEN_MEDIA_EVENT, onEvent);
    }
    // Re-open and take the route entry (the dismiss closed the chain).
    const menu = await openCanvasMenu();
    fireEvent.click(within(menu).getByText("Open Media"));
    const submenu = await screen.findByRole("menu", { name: "Open Media" });
    fireEvent.click(within(submenu).getByText("Open network stream…"));
    expect(await screen.findByText("network-stream-screen")).toBeInTheDocument();
  });

  it("Escape closes the menu chain; outside pointer dismisses", async () => {
    renderWithClient(<PlayerScreen />);
    const menu = await openCanvasMenu();
    fireEvent.keyDown(within(menu).getByText("Play"), { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    await openCanvasMenu();
    act(() => {
      fireEvent.pointerDown(document.body);
    });
    expect(screen.queryByRole("menu")).toBeNull();
  });
});
