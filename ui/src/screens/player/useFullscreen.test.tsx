// Card 4.C1 — fullscreen state-sync tests (doc 08 §4.3, doc 05 §2.7).
// jsdom implements none of the Fullscreen API — the mocks below define it,
// and the hook's honesty contract is asserted against the EVENTS, never the
// request return values.
import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useRef } from "react";
import { FullscreenUnsupportedError, useFullscreen } from "./useFullscreen";

function Host() {
  const ref = useRef<HTMLDivElement>(null);
  const fs = useFullscreen(ref);
  return (
    <div>
      <div ref={ref} data-testid="target" />
      <span data-fullscreen={String(fs.isFullscreen)} data-supported={String(fs.supported)} />
      <button type="button" data-testid="request" onClick={() => void fs.request()} />
      <button type="button" data-testid="exit" onClick={() => void fs.exit()} />
      <button type="button" data-testid="toggle" onClick={() => void fs.toggle()} />
    </div>
  );
}

function setFullscreenElement(element: Element | null): void {
  Object.defineProperty(document, "fullscreenElement", {
    value: element,
    configurable: true,
  });
  document.dispatchEvent(new Event("fullscreenchange"));
}

function statusOf(attr: "data-fullscreen" | "data-supported"): string {
  const el = document.querySelector(`[${attr}]`);
  expect(el).not.toBeNull();
  return el!.getAttribute(attr)!;
}

beforeEach(() => {
  vi.restoreAllMocks();
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
});

describe("useFullscreen (event-synced, never assumed)", () => {
  it("reports not-fullscreen and unsupported on a bare host", () => {
    Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
      value: undefined,
      configurable: true,
    });
    render(<Host />);
    expect(statusOf("data-fullscreen")).toBe("false");
    expect(statusOf("data-supported")).toBe("false");
  });

  it("requests fullscreen on the container; state syncs only from fullscreenchange", () => {
    render(<Host />);
    const target = screen.getByTestId("target");
    fireEvent.click(screen.getByTestId("request"));
    expect(target.requestFullscreen).toHaveBeenCalledTimes(1);
    // request dispatched but no event yet — state must NOT assume success
    expect(statusOf("data-fullscreen")).toBe("false");

    act(() => {
      setFullscreenElement(target);
    });
    expect(statusOf("data-fullscreen")).toBe("true");
    expect(document.exitFullscreen).not.toHaveBeenCalled();
  });

  it("toggle() exits while the live document reports fullscreen", () => {
    render(<Host />);
    const target = screen.getByTestId("target");
    act(() => {
      setFullscreenElement(target);
    });
    fireEvent.click(screen.getByTestId("toggle"));
    expect(document.exitFullscreen).toHaveBeenCalledTimes(1);
  });

  it("exit() calls document.exitFullscreen and syncs only on the event", () => {
    render(<Host />);
    act(() => {
      setFullscreenElement(document.body);
    });
    fireEvent.click(screen.getByTestId("exit"));
    expect(document.exitFullscreen).toHaveBeenCalledTimes(1);
    // still fullscreen until the event says otherwise
    expect(statusOf("data-fullscreen")).toBe("true");
    act(() => {
      setFullscreenElement(null);
    });
    expect(statusOf("data-fullscreen")).toBe("false");
  });

  it("rejects request/exit with FullscreenUnsupportedError on a bare host", async () => {
    Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
      value: undefined,
      configurable: true,
    });
    const { result } = renderHook(() => {
      const ref = useRef<HTMLDivElement>(null);
      return useFullscreen(ref);
    });
    await expect(result.current.request()).rejects.toBeInstanceOf(FullscreenUnsupportedError);
    await expect(result.current.exit()).rejects.toBeInstanceOf(FullscreenUnsupportedError);
  });
});
