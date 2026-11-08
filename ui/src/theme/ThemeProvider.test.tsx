// ThemeProvider tests — card 4.A2 AC: mode (dark/light/auto) + accent
// switching must land on the root <html> element as data-theme/data-accent
// attributes (doc 08 §3.3: "no JS style recalculation beyond attribute set").

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AccentId } from "../tokens/generated/tokens";
import { ThemeProvider, useTheme } from "./ThemeProvider";

// jsdom has no matchMedia — install a controllable stub.
function installMatchMedia(prefersLight: boolean) {
  const listeners = new Set<(event: { matches: boolean }) => void>();
  const query = {
    matches: prefersLight,
    addEventListener: (_: string, fn: (event: { matches: boolean }) => void) => {
      listeners.add(fn);
    },
    removeEventListener: (_: string, fn: (event: { matches: boolean }) => void) => {
      listeners.delete(fn);
    },
  };
  vi.stubGlobal("matchMedia", vi.fn().mockReturnValue(query));
  return {
    flip(next: boolean) {
      query.matches = next;
      for (const fn of listeners) fn({ matches: next });
    },
  };
}

function ModeProbe() {
  const { modeSetting, resolvedMode, accent, setModeSetting, setAccent } = useTheme();
  return (
    <div>
      <output data-testid="mode-setting">{modeSetting}</output>
      <output data-testid="resolved-mode">{resolvedMode}</output>
      <output data-testid="accent">{accent}</output>
      <button type="button" onClick={() => setModeSetting("light")}>
        set-light
      </button>
      <button type="button" onClick={() => setModeSetting("auto")}>
        set-auto
      </button>
      <button type="button" onClick={() => setAccent("violet")}>
        set-violet
      </button>
    </div>
  );
}

// Captures the context value so tests can call setters directly (a throw
// inside a click handler is re-thrown asynchronously by React's event
// dispatch, which would escape the assertion).
let capturedContext: ReturnType<typeof useTheme> | null = null;
function ContextCapture() {
  capturedContext = useTheme();
  return null;
}

const root = () => document.documentElement;

function renderProbe() {
  render(
    <ThemeProvider>
      <ModeProbe />
    </ThemeProvider>,
  );
}

beforeEach(() => {
  delete root().dataset.theme;
  delete root().dataset.accent;
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  delete root().dataset.theme;
  delete root().dataset.accent;
});

describe("ThemeProvider (doc 08 §3.3)", () => {
  it("defaults to dark + yellow (the product defaults, Solstice)", () => {
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
    expect(root().dataset.accent).toBe("yellow");
    expect(screen.getByTestId("resolved-mode").textContent).toBe("dark");
  });

  it("switching mode updates data-theme on <html>", () => {
    renderProbe();
    act(() => {
      screen.getByText("set-light").click();
    });
    expect(root().dataset.theme).toBe("light");
    expect(screen.getByTestId("mode-setting").textContent).toBe("light");
  });

  it("switching accent updates data-accent on <html>", () => {
    renderProbe();
    act(() => {
      screen.getByText("set-violet").click();
    });
    expect(root().dataset.accent).toBe("violet");
  });

  it("rejects an accent outside the generated 9", () => {
    render(
      <ThemeProvider>
        <ContextCapture />
      </ThemeProvider>,
    );
    expect(() =>
      capturedContext?.setAccent("magenta" as unknown as AccentId),
    ).toThrow(/unknown accent "magenta"/);
    expect(root().dataset.accent).toBe("yellow");
  });

  it("auto mode follows the OS preference and reacts to changes", () => {
    const media = installMatchMedia(true);
    renderProbe();
    act(() => {
      screen.getByText("set-auto").click();
    });
    expect(root().dataset.theme).toBe("light");

    act(() => {
      media.flip(false);
    });
    expect(root().dataset.theme).toBe("dark");
  });

  it("auto mode falls back to dark without matchMedia", () => {
    vi.stubGlobal("matchMedia", undefined);
    renderProbe();
    act(() => {
      screen.getByText("set-auto").click();
    });
    expect(root().dataset.theme).toBe("dark");
  });
});
