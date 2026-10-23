// Card 4.C3 — state-matrix tests (doc 08 §5.1–§5.4, doc 05 §1 C1–C8):
// every engine state × overlay shown/hidden; honest-indeterminate buffering
// (no fabricated %); unsupported shows real format info from the typed
// error; error is stage-tagged + actionable + retryable; empty first-run.
import { act, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CommandTransport, Event } from "../../../client/transport";
import type { CommandError, PlaybackState } from "../../../generated/commands";
import { usePlayerEngineView, type PlayerEngineView } from "./context-bridge";
import { resolveOverlay, StateRouter } from "./StateRouter";

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
  return render(<StateRouter view={v} onRetry={props.onRetry} onOpenMedia={props.onOpenMedia} />);
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

    rerender(<StateRouter view={view({ state: "error", error: err() })} />);
    const stub = screen.getByRole("button", { name: "Try again" });
    expect(stub).toBeDisabled(); // never offers a fake recovery
    expect(stub).toHaveAttribute("data-wiring-stub", "retry command dispatch — card 4.C2");
  });
});

// ------------------------------------------------------------ empty ---

describe("Empty surface — first-run (doc 08 §5.3)", () => {
  it("shows title, body, and a disabled Open-media affordance when unwired", () => {
    renderRouter(view({ state: "idle" }));
    expect(screen.getByRole("heading", { name: "No media open" })).toBeInTheDocument();
    expect(screen.getByText("Open media to start playback.")).toBeInTheDocument();
    const open = screen.getByRole("button", { name: "Open media" });
    expect(open).toBeDisabled();
    expect(open).toHaveAttribute("data-wiring-stub", "session.open — card 4.C2");
  });

  it("open-media fires when wired (C2 command dispatch)", () => {
    const onOpenMedia = vi.fn();
    renderRouter(view({ state: "idle" }), { onOpenMedia });
    screen.getByRole("button", { name: "Open media" }).click();
    expect(onOpenMedia).toHaveBeenCalledTimes(1);
  });
});

// ------------------------------------------------------------ context bridge ---

/** Minimal scripted transport for the bridge tests (never the MockTransport —
 * these need real event emission). */
class ScriptedTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();
  send(): Promise<never> {
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
});
