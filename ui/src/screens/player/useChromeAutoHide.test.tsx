// Card 4.C1 — chrome auto-hide state machine tests (doc 08 §4.3).
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
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

describe("useChromeAutoHide", () => {
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
