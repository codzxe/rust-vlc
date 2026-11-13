// Card 4.C3 — state-matrix tests (doc 08 §5.1–§5.4, doc 05 §1 C1–C8):
// every engine state × overlay shown/hidden; honest-indeterminate buffering
// (no fabricated %); unsupported shows real format info from the typed
// error; error is stage-tagged + actionable + retryable; empty first-run.
import { act, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import type { CommandTransport, CommandRequest, CommandResponse, Event } from "../../../client/transport";
import type { CommandError, PlaybackState, StateSnapshot } from "../../../generated/commands";
import { usePlayerEngineView, type PlayerEngineView } from "./context-bridge";
import { resolveOverlay, StateRouter } from "./StateRouter";
import { HomeOverlay } from "./StageOverlays";
import { PlaylistBrowse } from "./PlaylistBrowse";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const statesCss = readFileSync(`${dirname}/states.css`, "utf8");

const IDLE_VIEW: PlayerEngineView = {
  state: "idle",
  openingProgress: null,
  bufferingProgress: null,
  error: null,
};

function view(overrides: Partial<PlayerEngineView>): PlayerEngineView {
  return { ...IDLE_VIEW, ...overrides };
}

function err(overrides: Partial<CommandError> = {}): CommandError {
  return {
    code: "internal",
    details: {},
    message: "engine-reported failure",
    stage: "decode",
    ...overrides,
  };
}

function renderRouter(v: PlayerEngineView, props: { onRetry?: () => void; onOpenMedia?: () => void } = {}) {
  // w26.gfix-C: the idle surface reads the client (the demo transport's
  // remembered openings decide hero vs home), so the router tree mounts
  // under a ClientProvider like every real host (PlayerScreen's provider
  // chain). MockTransport is not the demo transport → no recents → the
  // first-run hero, exactly the bare-mount behavior the matrix pins.
  return render(
    <ClientProvider transport={new MockTransport()}>
      <StateRouter view={v} onRetry={props.onRetry} onOpenMedia={props.onOpenMedia} />
    </ClientProvider>,
  );
}

// ---------------------------------------------------------------- matrix ---

describe("StateRouter — engine state × overlay matrix (doc 08 §5.3)", () => {
  const cases: readonly [PlaybackState, string | null][] = [
    ["idle", "empty"],
    ["opening", "loading"],
    ["buffering", "buffering"],
    ["playing", null],
    ["paused", null],
    ["stopped", null],
    ["ended", null],
    ["error", "error"],
  ];

  for (const [state, expected] of cases) {
    it(`state=${state} → ${expected === null ? "no overlay" : `${expected} overlay`}`, () => {
      const v = view({ state });
      expect(resolveOverlay(v)).toBe(expected ?? "none");
      const { container } = renderRouter(v);
      const overlays = container.querySelectorAll("[data-overlay]");
      expect(overlays.length).toBeLessThanOrEqual(1); // zero or one, never stacked
      if (expected === null) {
        expect(overlays).toHaveLength(0);
      } else {
        expect(overlays[0]).toHaveAttribute("data-overlay", expected);
      }
    });
  }

  it("error state with typed unsupported code → unsupported overlay, not generic error", () => {
    const v = view({ state: "error", error: err({ code: "unsupported" }) });
    expect(resolveOverlay(v)).toBe("unsupported");
  });

  it("error state with unsupported-for-host code → unsupported overlay", () => {
    const v = view({ state: "error", error: err({ code: "unsupported-for-host" }) });
    expect(resolveOverlay(v)).toBe("unsupported");
  });
});

// ------------------------------------------------------------ honesty: progress ---

describe("Loading/Buffering — honest progress (never a fabricated %)", () => {
  it("loading with engine opening-progress renders determinate progressbar", () => {
    renderRouter(view({ state: "opening", openingProgress: 0.42 }));
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("data-progress", "determinate");
    expect(bar).toHaveAttribute("aria-valuenow", "42");
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "100");
  });

  it("loading without engine progress is indeterminate — no aria-valuenow, no fake number", () => {
    renderRouter(view({ state: "opening", openingProgress: null }));
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("data-progress", "indeterminate");
    expect(bar).not.toHaveAttribute("aria-valuenow");
    expect(bar.textContent).not.toMatch(/%/);
  });

  it("buffering with engine progress renders determinate progressbar", () => {
    renderRouter(view({ state: "buffering", bufferingProgress: 0.75 }));
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("data-progress", "determinate");
    expect(bar).toHaveAttribute("aria-valuenow", "75");
  });

  it("buffering without engine progress is indeterminate — honest label only", () => {
    renderRouter(view({ state: "buffering", bufferingProgress: null }));
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("data-progress", "indeterminate");
    expect(bar).not.toHaveAttribute("aria-valuenow");
    expect(bar.textContent).not.toMatch(/%/);
  });

  it("loading and buffering overlays announce busy state (aria-busy)", () => {
    renderRouter(view({ state: "opening" }));
    expect(screen.getByText("Opening media…")).toBeInTheDocument();
    expect(document.querySelector('[data-overlay="loading"]')).toHaveAttribute("aria-busy", "true");
  });
});

// ------------------------------------------------------------ unsupported ---

describe("Unsupported surface — real format info from the typed error (doc 08 §5.3)", () => {
  it("shows container/codec the error payload carries + no-playback-attempted wording", () => {
    renderRouter(
      view({
        state: "error",
        error: err({ code: "unsupported", details: { format: "matroska", codec: "V_VP9" } }),
      }),
    );
    expect(screen.getByRole("heading", { name: "Format not supported" })).toBeInTheDocument();
    expect(screen.getByText("matroska")).toBeInTheDocument();
    expect(screen.getByText("V_VP9")).toBeInTheDocument();
    expect(screen.getByText("No playback was attempted.")).toBeInTheDocument();
  });

  it("omits absent format keys — never invents a container/codec", () => {
    renderRouter(view({ state: "error", error: err({ code: "unsupported", details: { format: "mpegts" } }) }));
    expect(screen.getByText("mpegts")).toBeInTheDocument();
    expect(screen.queryByText("Codec")).toBeNull();
  });

  it("empty details → honest no-format message, no fabricated info", () => {
    renderRouter(view({ state: "error", error: err({ code: "unsupported", details: {} }) }));
    expect(
      screen.getByText("The engine reported no format details for this source."),
    ).toBeInTheDocument();
  });

  it("unknown detail keys are not surfaced (whitelist-only)", () => {
    renderRouter(
      view({ state: "error", error: err({ code: "unsupported", details: { path: "/home/u/secret.mkv", format: "matroska" } }) }),
    );
    expect(screen.getByText("matroska")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).toBeNull(); // never internal paths
  });
});

// ------------------------------------------------------------ error ---

describe("Error surface — stage-tagged, actionable, honest (doc 08 §5.4, doc 05 C5)", () => {
  it("renders the typed stage tag and the redacted Display message", () => {
    renderRouter(view({ state: "error", error: err({ stage: "demux", code: "corrupt" }) }));
    expect(screen.getByText("demux")).toBeInTheDocument();
    expect(screen.getByText("engine-reported failure")).toBeInTheDocument();
  });

  it("renders the actionable hint for the error code", () => {
    renderRouter(view({ state: "error", error: err({ code: "network" }) }));
    expect(screen.getByText("Check the connection, then try again.")).toBeInTheDocument();
  });

  it("renders the engine message verbatim — no internal paths, no invented detail", () => {
    renderRouter(view({ state: "error", error: err({ stage: "probe", code: "inaccessible", message: "Source is not readable" }) }));
    expect(screen.getByText("Source is not readable")).toBeInTheDocument();
    expect(screen.queryByText(/\/home\//)).toBeNull();
  });

  it("error state without a payload → honest unknown, no fabricated stage/code", () => {
    renderRouter(view({ state: "error", error: null }));
    expect(
      screen.getByText("Playback failed. The engine provided no error details."),
    ).toBeInTheDocument();
    expect(screen.getByText("unknown")).toBeInTheDocument();
  });

  it("retry fires when a real action is wired; disabled honest stub otherwise", () => {
    const onRetry = vi.fn();
    const { rerender } = renderRouter(view({ state: "error", error: err() }), { onRetry });
    const retry = screen.getByRole("button", { name: "Try again" });
    expect(retry).toBeEnabled();
    expect(retry).not.toHaveAttribute("data-wiring-stub");
    retry.click();
    expect(onRetry).toHaveBeenCalledTimes(1);

    rerender(
      <ClientProvider transport={new MockTransport()}>
        <StateRouter view={view({ state: "error", error: err() })} />
      </ClientProvider>,
    );
    const stub = screen.getByRole("button", { name: "Try again" });
    expect(stub).toBeDisabled(); // never offers a fake recovery
    expect(stub).toHaveAttribute("data-wiring-stub", "retry command dispatch — card 4.C2");
  });
});

// ------------------------------------------------------------ empty ---

describe("Empty surface — first-run (doc 08 §5.3)", () => {
  it("shows title, body, and the Open-media affordance wired to the shell dialog event", () => {
    // Solstice redesign contract change: the button dispatches the shell-wide
    // `lumen:open-media` window event (the app-shell file dialog listens for
    // exactly that name) instead of rendering as a disabled wiring stub —
    // the assertion now pins the real dispatch path.
    renderRouter(view({ state: "idle" }));
    expect(screen.getByRole("heading", { name: "No media open" })).toBeInTheDocument();
    expect(screen.getByText("Open media to start playback.")).toBeInTheDocument();
    const open = screen.getByRole("button", { name: "Open media" });
    expect(open).toBeEnabled();
    expect(open).not.toHaveAttribute("data-wiring-stub");
    const heardOpenMedia = vi.fn();
    window.addEventListener("lumen:open-media", heardOpenMedia);
    open.click();
    expect(heardOpenMedia).toHaveBeenCalledTimes(1);
    window.removeEventListener("lumen:open-media", heardOpenMedia);
  });

  it("open-media fires when wired (C2 command dispatch)", () => {
    const onOpenMedia = vi.fn();
    renderRouter(view({ state: "idle" }), { onOpenMedia });
    screen.getByRole("button", { name: "Open media" }).click();
    expect(onOpenMedia).toHaveBeenCalledTimes(1);
  });

  it("renders the onboarding cone mark as decorative inline art (W26 hero)", () => {
    // The hero's cone is the frame-exact Onboarding.svg port — decorative
    // (aria-hidden) inline SVG carrying no text alternative (the heading
    // carries the semantics), filled by the accent token via currentColor.
    const { container } = renderRouter(view({ state: "idle" }));
    const mark = container.querySelector(".lumen-state-hero__mark");
    expect(mark).not.toBeNull();
    expect(mark).toHaveAttribute("aria-hidden", "true");
    expect(mark?.querySelectorAll("path").length).toBe(3); // plinth + band + tip
  });
});

// ------------------------------------------------------------ home (Player-5.svg, w26.gfix-C g04 P1-7) ---

/** The frame's section headers, y-sorted (Continue Watching y≈157 → Listen
 * Again y≈505 → Frequently Visited Folders y≈894 @1920). */
const HOME_SECTIONS = ["Continue Watching", "Listen Again", "Frequently Visited Folders"] as const;

describe("Home surface — Player-5.svg's three sections over real remembered openings", () => {
  const recents = [
    { source: { kind: "file", path: "/media/The_Wand_S01E05.mp4" }, title: "The_Wand_S01E05.mp4" },
    { source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" }, title: "Big Buck Bunny" },
    { source: { kind: "file", path: "/media/demo/ambient-loop.flac" }, title: "Ambient loop" },
    { source: { kind: "url", url: "https://ice1.somafm.com/groovesalad-128-mp3" }, title: "ice1.somafm.com" },
  ] as const;

  const renderHome = () =>
    render(
      <ClientProvider transport={new MockTransport()}>
        <HomeOverlay recents={recents} />
      </ClientProvider>,
    );

  it("renders the three frame sections in y-sorted order", () => {
    const { container } = renderHome();
    const headers = [...container.querySelectorAll<HTMLElement>(".lumen-state-home__header")].map(
      (header) => header.textContent,
    );
    expect(headers).toEqual([...HOME_SECTIONS]);
    const root = container.querySelector(".lumen-state-home");
    expect(root).toHaveAttribute("data-overlay", "empty");
    expect(root).toHaveAttribute("data-empty-state", "home");
  });

  it("splits the REAL openings by kind: videos under Continue Watching, audio under Listen Again", () => {
    renderHome();
    // Video cards: the .mp4 + the .mkv.
    const videos = screen.getAllByRole("button", { name: /^Play / }).map((card) => card.getAttribute("aria-label"));
    expect(videos).toContain("Play The_Wand_S01E05.mp4");
    expect(videos).toContain("Play Big Buck Bunny");
    expect(videos).toContain("Play Ambient loop");
    expect(videos).toContain("Play ice1.somafm.com");
    // The kind split is the frame's two card rows: .mp4/.mkv are video
    // cards (237×133), the .flac and the radio stream are album squares.
    const albumCells = document.querySelectorAll(".lumen-state-home__cell--album");
    expect(albumCells).toHaveLength(2);
    expect(document.querySelectorAll(".lumen-state-home__card:not([data-card-kind='album'])")).toHaveLength(2);
  });

  it("Frequently Visited Folders renders the honest empty face — no fabricated folders", () => {
    renderHome();
    const sections = [...document.querySelectorAll<HTMLElement>(".lumen-state-home__section")];
    const folders = sections[2];
    expect(folders?.querySelector(".lumen-state-home__empty")?.textContent).toBe("Nothing here yet");
    expect(folders?.querySelectorAll(".lumen-state-home__card").length).toBe(0);
  });

  it("cards never fabricate watch progress — no progress bars without a resume seam", () => {
    const { container } = renderHome();
    // Player-5.svg pins 4px #D37918 bars per card; the engine reports no
    // resume positions, so the home ships none (honest omission).
    expect(container.querySelectorAll("[data-progress]").length).toBe(0);
  });

  it("an empty kind renders its honest empty note (never a fabricated row)", () => {
    render(
      <ClientProvider transport={new MockTransport()}>
        <HomeOverlay recents={[{ source: { kind: "file", path: "/a/movie.mkv" }, title: "movie.mkv" }]} />
      </ClientProvider>,
    );
    const sections = [...document.querySelectorAll<HTMLElement>(".lumen-state-home__section")];
    expect(sections[1]?.querySelector(".lumen-state-home__empty")?.textContent).toBe("Nothing here yet");
    expect(screen.getAllByRole("button", { name: /^Play / })).toHaveLength(1);
  });
});

// ------------------------------------------------------------ states.css skin (W26) ---

describe("states.css — the W26 Figma skin (figma-fidelity-spec §1–§2)", () => {
  it("overlays sit on a dark scrim; failure cards are r12 panels with hairline + shadow", () => {
    expect(statesCss).toMatch(
      /\.lumen-state-overlay\s*\{[^}]*color-mix\(in srgb, var\(--color-surface-deep\) 72%, transparent\)/,
    );
    expect(statesCss).toMatch(
      /\.lumen-state-overlay__cluster\[data-card\]\s*\{[^}]*--color-surface-panel[^}]*--color-stroke-border[^}]*--radius-panel[^}]*--shadow-overlay/,
    );
  });

  it("loading/buffering: thin orange arc spinner + slim orange bar on a deep track", () => {
    expect(statesCss).toMatch(
      /\.lumen-state-overlay__spinner\s*\{[^}]*--color-accent\) 32%[^}]*border-top-color: var\(--color-accent\)/,
    );
    expect(statesCss).toMatch(
      /\.lumen-state-overlay__bar\s*\{[^}]*--color-surface-deep[^}]*--radius-ring/,
    );
    expect(statesCss).toMatch(
      /\.lumen-state-overlay__fill\s*\{[^}]*--color-accent/,
    );
  });

  it("hero: accent cone, display-scale regular heading, secondary body — onboarding grammar", () => {
    expect(statesCss).toMatch(
      /\.lumen-state-hero__mark\s*\{[^}]*width: 96px[^}]*--color-accent/,
    );
    expect(statesCss).toMatch(
      /\.lumen-state-hero__title\s*\{[^}]*--color-content-bright[^}]*--font-size-display[^}]*--font-weight-regular/,
    );
    expect(statesCss).toMatch(
      /\.lumen-state-hero__body\s*\{[^}]*--color-content-secondary/,
    );
    // The hero CTA is the shared primary button (r6, button.css) — no
    // per-surface radius override may fight the shared control radius.
    expect(statesCss).not.toMatch(/\.lumen-state-hero[^{]*\.lumen-button[^{]*\{/);
  });

  it("hide-UI (Player - hide UI.svg): transient overlays fade with the hidden chrome; alerts never do", () => {
    // Loading + buffering fade when the screen chrome hides (idle cursor)…
    expect(statesCss).toMatch(
      /\.lumen-player\[data-chrome="hidden"\] \.lumen-state-overlay\[data-overlay="loading"\],\s*\n\.lumen-player\[data-chrome="hidden"\] \.lumen-state-overlay\[data-overlay="buffering"\]\s*\{[^}]*opacity: 0/,
    );
    // …but the selector must NOT reach the failure surfaces (role=alert) or
    // the empty hero (the hero is canvas content, not chrome — w26.f10 F4:
    // the chrome AROUND it now fades on idle, the hero itself never does).
    expect(statesCss).not.toMatch(/\[data-chrome="hidden"\][^{]*data-overlay="error"/);
    expect(statesCss).not.toMatch(/\[data-chrome="hidden"\][^{]*data-overlay="unsupported"/);
    expect(statesCss).not.toMatch(/\[data-chrome="hidden"\][^{]*lumen-state-hero/);
  });

  it("light mode keeps the overlay scrim + on-scrim ink DARK (picture-frame rule, e19 F7)", () => {
    // The transient overlays sit ON the stays-dark canvas; the scrim must
    // not flip light with the theme — light mode pins the dark theme's
    // frame values (same precedent as the hero pins below).
    expect(statesCss).toMatch(
      /:root\[data-theme="light"\] \.lumen-state-overlay\s*\{[^}]*#0e0e0e 72%, transparent/,
    );
    expect(statesCss).toMatch(
      /:root\[data-theme="light"\] \.lumen-state-overlay\s*\{[^}]*color: #c6c6c6/,
    );
    expect(statesCss).toMatch(
      /:root\[data-theme="light"\] \.lumen-state-overlay__label\s*\{[^}]*color: #8f8f8f/,
    );
    expect(statesCss).toMatch(
      /:root\[data-theme="light"\] \.lumen-state-overlay__bar\s*\{[^}]*background: #0e0e0e/,
    );
    // Failure cards stay THEMED (Figma panel grammar flips with the theme) —
    // only the canvas-side scrim + its direct ink pin dark.
    expect(statesCss).not.toMatch(
      /:root\[data-theme="light"\] \.lumen-state-overlay__cluster\[data-card\]/,
    );
  });

  it("motion is reduced-motion safe: spinner spin + hide-UI fade are motion-gated", () => {
    expect(statesCss).toMatch(
      /@media \(prefers-reduced-motion: no-preference\)\s*\{[^}]*\.lumen-state-overlay__spinner\s*\{[^}]*animation: lumen-state-spin/,
    );
    expect(statesCss).toMatch(
      /@media \(prefers-reduced-motion: reduce\)\s*\{[^}]*\.lumen-state-overlay\s*\{[^}]*transition: none/,
    );
  });
});

// --------------------------- browse grid keys (w26.k06, J06-7 P2) ---

describe("PlaylistBrowse — duplicate item names (w26.k06, J06-7)", () => {
  it("duplicate filenames render as DISTINCT cells (no React key collision)", () => {
    // Saved playlists carry NAMES only (playlistsStore — no uniqueness
    // guarantee): the wave-J review found the grid keyed by name alone, so
    // duplicate filenames collide. The index-prefixed key must render all
    // cells with a clean console (React logs "Encountered two children with
    // the same key" on collision — the honest probe for this regression).
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      render(
        <PlaylistBrowse
          playlist={{ id: "dupes", title: "Dupes", itemNames: ["clip.mp4", "clip.mp4", "other.mp4"] }}
          hasMedia={false}
          previewOpen={true}
          onPreviewToggle={() => {}}
          onExit={() => {}}
        />,
      );
      expect(screen.getAllByRole("listitem")).toHaveLength(3);
      const keyWarnings = errorSpy.mock.calls.filter((args) =>
        String(args[0] ?? "").includes("same key"),
      );
      expect(keyWarnings).toHaveLength(0);
    } finally {
      errorSpy.mockRestore();
    }
  });
});

// ------------------------------------------------------------ context bridge ---

/** Minimal valid snapshot body (only `state` varies per test). */
const SNAPSHOT_BASE: StateSnapshot = {
  audioDelayMs: 0,
  devices: [],
  duration: { kind: "unknown" },
  muted: false,
  playlist: {},
  rate: 1,
  selections: {},
  state: "idle",
  subtitle: {},
  subtitleDelayMs: 0,
  time: null,
  tracks: [],
  videoDelayMs: 0,
  volume: 0.8,
};

/** Scripted transport for the bridge tests (never the MockTransport —
 * these need real event emission + a controllable snapshot answer). */
class ScriptedTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();
  /** Requests issued through send() (asserts the mount-time probe). */
  readonly sent: CommandRequest[] = [];
  /** The engine's answer to system.get-state-snapshot (default: idle). */
  snapshotState: PlaybackState = "idle";
  /** When set, the snapshot response waits for this gate (ordering tests). */
  snapshotGate: Promise<void> | null = null;

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    if (request.cmd === "system.get-state-snapshot") {
      if (this.snapshotGate !== null) {
        await this.snapshotGate;
      }
      return {
        gen: 1,
        requestId: request.requestId,
        result: {
          completed: {
            kind: "state-snapshot",
            snapshot: { ...SNAPSHOT_BASE, state: this.snapshotState },
          },
        },
      };
    }
    return Promise.reject(new Error("not used in this test"));
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  emit(event: Event): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }
}

let captured: PlayerEngineView | null = null;

function ViewCapture({ transport }: { transport?: CommandTransport | null }): null {
  captured = usePlayerEngineView(transport);
  return null;
}

describe("context-bridge — engine events reduce into the view (doc 05 C4)", () => {
  it("tracks state changes, progress, and the fatal error payload", () => {
    const transport = new ScriptedTransport();
    render(<ViewCapture transport={transport} />);

    expect(captured).toEqual(IDLE_VIEW); // no engine word yet → honest idle

    act(() => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "opening" });
      transport.emit({ seq: 2, kind: "session.opening-progress", progress: 0.5 });
    });
    expect(captured).toEqual(view({ state: "opening", openingProgress: 0.5 }));

    act(() => {
      transport.emit({ seq: 3, kind: "session.state-changed", from: "opening", to: "buffering" });
      transport.emit({ seq: 4, kind: "session.buffering", progress: 0.25 });
    });
    expect(captured).toEqual(
      view({ state: "buffering", openingProgress: null, bufferingProgress: 0.25 }),
    );

    act(() => {
      transport.emit({
        seq: 5,
        kind: "playback.error",
        fatal: true,
        code: "corrupt",
        details: {},
        message: "engine-reported failure",
        stage: "demux",
      });
      transport.emit({ seq: 6, kind: "session.state-changed", from: "buffering", to: "error" });
    });
    expect(captured?.state).toBe("error");
    expect(captured?.error?.code).toBe("corrupt"); // payload preserved for the stage-tagged surface

    act(() => {
      transport.emit({ seq: 7, kind: "session.state-changed", from: "error", to: "idle" });
    });
    expect(captured).toEqual(IDLE_VIEW); // close clears stale evidence (C6)
  });

  it("without a transport it stays honestly idle (no fabricated state)", () => {
    const { unmount } = render(<ViewCapture />);
    expect(captured).toEqual(IDLE_VIEW);
    unmount();
  });

  it("issues the mount-time snapshot probe and never seeds from failures", async () => {
    // Scaffold transports answer `feature-not-exposed` (MockTransport) or
    // reject outright — the bridge must leave the honest idle view stand.
    const transport = new ScriptedTransport();
    transport.snapshotGate = Promise.reject(new Error("wire down"));
    render(<ViewCapture transport={transport} />);
    expect(transport.sent.some((request) => request.cmd === "system.get-state-snapshot")).toBe(true);
    await act(async () => {});
    expect(captured).toEqual(IDLE_VIEW);
  });
});

describe("context-bridge — engine truth survives player-route remounts (w26.f10 F3, e03 F3)", () => {
  it("seeds the session state from the mount-time engine snapshot", async () => {
    // Remount against an already-active engine (e.g. media kept playing
    // while the user was on /playlist): a pure event-reducer would restart
    // at idle and render the empty hero over playing media.
    const transport = new ScriptedTransport();
    transport.snapshotState = "playing";
    render(<ViewCapture transport={transport} />);
    expect(transport.sent.some((request) => request.cmd === "system.get-state-snapshot")).toBe(true);
    await act(async () => {});
    expect(captured?.state).toBe("playing");
    // Only the session state seeds — progress stays event-only honest.
    expect(captured?.openingProgress).toBeNull();
    expect(captured?.bufferingProgress).toBeNull();
    expect(captured?.error).toBeNull();
  });

  it("a remount over the same wire starts from the cached engine view (no idle flash)", () => {
    const transport = new ScriptedTransport();
    transport.snapshotState = "playing";
    const first = render(<ViewCapture transport={transport} />);
    act(() => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "playing" });
    });
    expect(captured?.state).toBe("playing");
    first.unmount();
    // The very FIRST render of the remount already carries engine truth.
    render(<ViewCapture transport={transport} />);
    expect(captured?.state).toBe("playing");
  });

  it("a fresh transport (different wire) honestly starts from idle", () => {
    const transportA = new ScriptedTransport();
    transportA.snapshotState = "playing";
    const first = render(<ViewCapture transport={transportA} />);
    act(() => {
      transportA.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "playing" });
    });
    first.unmount();
    // A different transport instance = a different engine: no leakage.
    const transportB = new ScriptedTransport();
    render(<ViewCapture transport={transportB} />);
    expect(captured?.state).toBe("idle");
  });

  it("live session.state-changed beats a late snapshot response", async () => {
    const transport = new ScriptedTransport();
    transport.snapshotState = "playing"; // queried before the pause below
    let release!: () => void;
    transport.snapshotGate = new Promise<void>((resolve) => {
      release = resolve;
    });
    render(<ViewCapture transport={transport} />);
    act(() => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "playing", to: "paused" });
    });
    expect(captured?.state).toBe("paused");
    release();
    await act(async () => {});
    // The stale in-flight snapshot must NOT regress the live truth.
    expect(captured?.state).toBe("paused");
  });

  it("a remount's snapshot correction clears the cached state's transient evidence (C6)", async () => {
    const transport = new ScriptedTransport();
    const first = render(<ViewCapture transport={transport} />);
    act(() => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "buffering" });
      transport.emit({ seq: 2, kind: "session.buffering", progress: 0.25 });
    });
    first.unmount();
    // While unmounted the engine moved on — the remount probe says playing.
    transport.snapshotState = "playing";
    render(<ViewCapture transport={transport} />);
    expect(captured?.state).toBe("buffering"); // cached: no flash
    expect(captured?.bufferingProgress).toBe(0.25);
    await act(async () => {});
    expect(captured?.state).toBe("playing");
    expect(captured?.bufferingProgress).toBeNull(); // stale progress cleared
  });
});
