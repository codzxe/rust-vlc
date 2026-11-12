// Card 4.C1 — chrome auto-hide state machine tests (doc 08 §4.3).
//
// Two contracts (w26.f10 F4): the LEGACY standalone machine (no provider —
// `enabled` gates arming, `locked` locks as passed) and the PLAYER
// COMPOSITION semantics (PlayerControlsProvider above): armed windowed AND
// fullscreen, empty state included; `locked` honored only while the engine
// reports media open (the screen's legacy call folds the retired no-media
// gate into `locked`).
import { act, renderHook } from "@testing-library/react";
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
