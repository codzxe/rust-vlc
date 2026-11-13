// Card 4.C1 — chrome auto-hide state machine tests (doc 08 §4.3).
//
// Two contracts (w26.f10 F4): the LEGACY standalone machine (no provider —
// `enabled` gates arming, `locked` locks as passed) and the PLAYER
// COMPOSITION semantics (PlayerControlsProvider above): armed windowed AND
// fullscreen, empty state included. w26.gfix-F (g06 P1-2) adds the
// DOM-truth overlay lock on top of both: an open menu or modal dialog in
// the document locks the chrome visible.
import { act, render, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandTransport, Event } from "../../client/transport";
import { PlayerControlsProvider } from "./context";
import { CHROME_AUTO_HIDE_DELAY_MS, useChromeAutoHide } from "./useChromeAutoHide";

interface Props {
  enabled: boolean;
  locked: boolean;
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useChromeAutoHide — legacy standalone machine (no player seam above)", () => {
  it("starts visible and stays visible while auto-hide is disabled (windowed)", () => {
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: false, locked: false },
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS * 3);
    });
    expect(result.current.visibility).toBe("visible");
  });

  it("hides after the idle delay and reveals on pointer activity", () => {
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS - 1);
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.visibility).toBe("hidden");

    act(() => {
      window.dispatchEvent(new Event("pointermove"));
    });
    expect(result.current.visibility).toBe("visible");
    // reveal restarts the idle timer
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS - 1);
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.visibility).toBe("hidden");
  });

  it("reveals on keydown as well (any activity shows)", () => {
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "a" }));
    });
    expect(result.current.visibility).toBe("visible");
  });

  it("locks visible while a menu/panel is open — the timer never fires", () => {
    const { result, rerender } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: true },
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS * 10);
    });
    expect(result.current.visibility).toBe("locked");

    // unlocking re-arms the idle timer
    rerender({ enabled: true, locked: false });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
  });

  it("returns to visible immediately when auto-hide turns off", () => {
    const { result, rerender } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
    rerender({ enabled: false, locked: false });
    expect(result.current.visibility).toBe("visible");
  });

  it("show() reveals and restarts the timer (keymap show-controls action)", () => {
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
    act(() => {
      result.current.show();
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS - 1);
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.visibility).toBe("hidden");
  });
});

// ------------------------------------------------------------ player seam ---

/** Scripted transport: emits engine events on demand; commands reject (the
 * provider's probes are try/caught — only the event stream matters here). */
class ScriptedTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();
  send(): Promise<never> {
    return Promise.reject(new Error("no commands needed in this test"));
  }
  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }
}

/** Mounts the hook inside the real player composition seam. */
function playerWrapper(transport: CommandTransport) {
  return function Wrapper({ children }: { children: ReactNode }): ReactNode {
    return (
      <ClientProvider transport={transport}>
        <PlayerControlsProvider>{children}</PlayerControlsProvider>
      </ClientProvider>
    );
  };
}

describe("useChromeAutoHide — player composition (w26.f10 F4, e03 F4)", () => {
  it("fades chrome on cursor idle WINDOWED while media plays (frame: hide-UI r19 window stroke)", () => {
    const transport = new ScriptedTransport();
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: false, locked: false },
      wrapper: playerWrapper(transport),
    });
    act(() => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "playing" });
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
  });

  it("fades chrome in the EMPTY state too — the no-media gate is retired", () => {
    // The screen's call passes locked=panelsOnly (w26.f10 cleanup): with no
    // panel up, the empty hero is canvas content (never fades — states.css
    // pins it), the chrome around it does. This is the /player first-run
    // hide-UI state.
    const transport = new ScriptedTransport();
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: false, locked: false },
      wrapper: playerWrapper(transport),
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS * 2);
    });
    expect(result.current.visibility).toBe("hidden");
    act(() => {
      window.dispatchEvent(new Event("pointermove"));
    });
    expect(result.current.visibility).toBe("visible");
  });

  it("still locks while a panel is open AND media is playing (doc 08 §4.5)", () => {
    const transport = new ScriptedTransport();
    const { result, rerender } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: true },
      wrapper: playerWrapper(transport),
    });
    act(() => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "playing" });
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS * 5);
    });
    expect(result.current.visibility).toBe("locked");
    // closing the panel re-arms the idle timer
    rerender({ enabled: true, locked: false });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
  });

  it("media closing (engine back to idle) re-arms the fade for the empty state", () => {
    const transport = new ScriptedTransport();
    const { result, rerender } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
      wrapper: playerWrapper(transport),
    });
    act(() => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "playing" });
    });
    act(() => {
      transport.emit({ seq: 2, kind: "session.state-changed", from: "playing", to: "idle" });
    });
    // no panel is open — locked stays false (the retired !hasMedia gate is
    // gone from the call site), so the idle fade re-arms for the empty state
    rerender({ enabled: true, locked: false });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
  });
});

// ------------------------------------------- open-overlay lock (g06 P1-2) ---

describe("useChromeAutoHide — open-overlay lock (w26.gfix-F, g06 P1-2)", () => {
  /** Mounts an overlay node (present in DOM === open, the seam contract). */
  const mountOverlay = (role: "menu" | "dialog"): HTMLDivElement => {
    const node = document.createElement("div");
    node.setAttribute("role", role);
    document.body.appendChild(node);
    return node;
  };

  it("locks visible while a menu is open in the document — the timer never fires", async () => {
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");

    // a menu opens (the writer lives in another lane — DOM truth only):
    // the hidden chrome must reveal and lock, never re-fade under it.
    let menu: HTMLDivElement | null = null;
    await act(async () => {
      menu = mountOverlay("menu");
    });
    expect(result.current.visibility).toBe("locked");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS * 10);
    });
    expect(result.current.visibility).toBe("locked");

    // closing it re-arms the idle timer
    await act(async () => {
      menu?.remove();
    });
    expect(result.current.visibility).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
  });

  it("locks for an open modal dialog too (the same stranding argument)", async () => {
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
    });
    let dialog: HTMLDivElement | null = null;
    await act(async () => {
      dialog = mountOverlay("dialog");
    });
    expect(result.current.visibility).toBe("locked");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS * 5);
    });
    expect(result.current.visibility).toBe("locked");
    await act(async () => {
      dialog?.remove();
    });
    expect(result.current.visibility).toBe("visible");
  });

  it("an overlay opened while hidden reveals first, then locks", async () => {
    const { result } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: true, locked: false },
    });
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS);
    });
    expect(result.current.visibility).toBe("hidden");
    await act(async () => {
      mountOverlay("menu");
    });
    // revealed AND locked in one transition — a stale hidden never survives
    expect(result.current.visibility).toBe("locked");
    document.querySelector('[role="menu"]')?.remove();
  });
});

// ------------------------------- render-cost contract (w26.k06, J06-1 P1) ---

describe("useChromeAutoHide — activity costs no renders (w26.k06, J06-1)", () => {
  /** Host render count — the probe for the wave-I regression: the
   * activity-nonce fix re-rendered the host (PlayerScreen, the app's
   * biggest tree) on EVERY pointer event; the ref-held scheduler must keep
   * the i02 restart semantics with ZERO renders per event. */
  let hostRenders = 0;

  function RenderCountingHost(props: Props) {
    hostRenders += 1;
    const chrome = useChromeAutoHide(props);
    return <output data-testid="chrome-visibility">{chrome.visibility}</output>;
  }

  beforeEach(() => {
    hostRenders = 0;
  });

  it("a pointermove storm while visible re-renders NOTHING (no setState per event)", () => {
    render(<RenderCountingHost enabled={true} locked={false} />);
    expect(hostRenders).toBeGreaterThanOrEqual(1);
    const baseline = hostRenders;

    // A 25-event burst — the old nonce impl rendered once per event (25
    // renders); the ref-held scheduler must render exactly zero times.
    act(() => {
      for (let i = 0; i < 25; i++) {
        window.dispatchEvent(new Event("pointermove"));
      }
    });
    expect(hostRenders).toBe(baseline);

    // Chrome is still visible, and the countdown was RESTARTED by the last
    // event (the i02 semantics the nonce bought — retained): delay-1 after
    // the burst it is still visible, +1ms it hides.
    const visibility = document.querySelector('[data-testid="chrome-visibility"]');
    expect(visibility?.textContent).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS - 1);
    });
    expect(visibility?.textContent).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(visibility?.textContent).toBe("hidden");
  });

  it("activity while visible defers the hide (restart semantics, mid-burst check)", () => {
    render(<RenderCountingHost enabled={true} locked={false} />);
    const visibility = document.querySelector('[data-testid="chrome-visibility"]');
    // Halfway through the delay, activity arrives; the full delay must run
    // FROM THE EVENT, not the original arm.
    act(() => {
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS / 2);
      window.dispatchEvent(new Event("pointermove"));
      vi.advanceTimersByTime(CHROME_AUTO_HIDE_DELAY_MS - 1);
    });
    expect(visibility?.textContent).toBe("visible");
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(visibility?.textContent).toBe("hidden");
  });

  it("w26.k06 (J06-3): the returned object is referentially stable across renders", () => {
    const { result, rerender } = renderHook((props: Props) => useChromeAutoHide(props), {
      initialProps: { enabled: false, locked: false },
    });
    const first = result.current;
    // A forced host re-render (same props) must NOT mint a fresh object —
    // consumers key effect deps on it (PlayerScreen's keymap listener).
    rerender({ enabled: false, locked: false });
    expect(result.current).toBe(first);
  });
});
