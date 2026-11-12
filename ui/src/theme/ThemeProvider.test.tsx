// ThemeProvider tests — card 4.A2 AC: mode (dark/light/auto) + accent
// switching must land on the root <html> element as data-theme/data-accent
// attributes (doc 08 §3.3: "no JS style recalculation beyond attribute set").
// w26.f08 (e03-F1 / e19-F1): the persisted `interface.theme` override must
// hydrate on the FIRST render (the sync hook used to mount only inside the
// settings screens, so every other route booted dark regardless of the
// saved choice).

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AccentId } from "../tokens/generated/tokens";
import { ThemeProvider } from "./ThemeProvider";
import { useTheme } from "./useTheme";

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
  // The provider lazy-initializes from the persisted settings overrides
  // (w26.f08) — start every test with a clean store so cases stay isolated.
  window.localStorage.clear();
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
  it("defaults to dark + orange (the product defaults, W25)", () => {
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
    expect(root().dataset.accent).toBe("orange");
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
    expect(root().dataset.accent).toBe("orange");
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

describe("ThemeProvider — persisted interface.theme hydration (w26.f08)", () => {
  /** Seeds the settings overrides store exactly like the settings screen's
   * persistence seam writes it (lumen.settings.overrides.v1). */
  function seedThemeOverride(value: unknown): void {
    window.localStorage.setItem(
      "lumen.settings.overrides.v1",
      JSON.stringify({ "interface.theme": value }),
    );
  }

  it("hydrates a persisted LIGHT choice on mount — the first render already carries it (e03-F1/e19-F1)", () => {
    seedThemeOverride("light");
    renderProbe();
    expect(root().dataset.theme).toBe("light");
    expect(screen.getByTestId("mode-setting").textContent).toBe("light");
    expect(screen.getByTestId("resolved-mode").textContent).toBe("light");
  });

  it("hydrates a persisted DARK choice even when the OS prefers light (the saved choice wins)", () => {
    installMatchMedia(true);
    seedThemeOverride("dark");
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
  });

  it("hydrates a persisted SYSTEM (auto) choice and follows the OS preference both ways (e19-F5)", () => {
    const media = installMatchMedia(true); // OS = light
    seedThemeOverride("auto");
    renderProbe();
    expect(root().dataset.theme).toBe("light");
    expect(screen.getByTestId("mode-setting").textContent).toBe("auto");

    act(() => {
      media.flip(false); // OS flips dark
    });
    expect(root().dataset.theme).toBe("dark");
  });

  it("hydrates a persisted SYSTEM (auto) choice against a dark-preferring OS", () => {
    installMatchMedia(false);
    seedThemeOverride("auto");
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
    expect(screen.getByTestId("mode-setting").textContent).toBe("auto");
  });

  it("keeps the dark default for absent, unknown or corrupt persisted values (never guesses)", () => {
    seedThemeOverride("solarized");
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
    cleanup();

    window.localStorage.setItem("lumen.settings.overrides.v1", "{not json");
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
    cleanup();

    window.localStorage.removeItem("lumen.settings.overrides.v1");
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
  });

  it("still defaults to dark + orange with an empty store (the product defaults stand)", () => {
    renderProbe();
    expect(root().dataset.theme).toBe("dark");
    expect(root().dataset.accent).toBe("orange");
  });
});
