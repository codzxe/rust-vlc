import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ThemeProvider } from "./ThemeProvider";
import {
  readPersistedAccent,
  readPersistedThemeMode,
  useSettingsThemeSync,
} from "./settingsThemeSync";
import { SettingsProvider, useSettings } from "../screens/settings/SettingsProvider";
import { memorySettingsPersistence } from "../screens/settings/persistence";
import { SETTING_GLOBAL_ACCENT_COLOR, SETTING_INTERFACE_THEME } from "../screens/settings/generated/schema";

// Card 4.E2 appearance wiring: setting change → ThemeProvider state → the
// data-theme attribute on <html> (doc 08 §3.3 attribute-only theming).
// w26.f08 round 2: the same contract covers data-accent (global.accent-color).

/** Mounts the sync seam plus a radio that writes interface.theme for real. */
function Harness({ mode, revertButton = false }: { mode: string; revertButton?: boolean }) {
  const { setValue, revert } = useSettings();
  return (
    <>
      <Sync />
      <input type="radio" data-testid={`set-${mode}`} onChange={() => setValue(SETTING_INTERFACE_THEME.key, mode)} />
      <input type="radio" data-testid="set-rogue" onChange={() => setValue(SETTING_INTERFACE_THEME.key, "solarized")} />
      <input type="radio" data-testid="set-accent-violet" onChange={() => setValue(SETTING_GLOBAL_ACCENT_COLOR.key, "violet")} />
      <input type="radio" data-testid="set-accent-rogue" onChange={() => setValue(SETTING_GLOBAL_ACCENT_COLOR.key, "magenta")} />
      {revertButton ? <button type="button" data-testid="revert" onClick={() => revert()} /> : null}
    </>
  );
}

function Sync() {
  useSettingsThemeSync();
  return null;
}

const mount = (ui: React.ReactNode) =>
  render(
    <ThemeProvider>
      <SettingsProvider persistence={memorySettingsPersistence}>{ui}</SettingsProvider>
    </ThemeProvider>,
  );

describe("readPersistedThemeMode — the bootstrap read (w26.f08, e03-F1/e19-F1)", () => {
  /** Writes overrides through the real localStorage seam (same key as Apply). */
  const seed = (value: unknown): void =>
    window.localStorage.setItem(
      "lumen.settings.overrides.v1",
      JSON.stringify({ "interface.theme": value }),
    );

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    delete document.documentElement.dataset.theme;
  });

  it("reads dark/light through the same persistence seam SettingsProvider hydrates from", () => {
    expect(readPersistedThemeMode()).toBeNull(); // clean store — never guesses
    seed("light");
    expect(readPersistedThemeMode()).toBe("light");
    seed("dark");
    expect(readPersistedThemeMode()).toBe("dark");
  });

  it("accepts SYSTEM (auto) — a persisted system choice is a valid mode (e19-F5)", () => {
    seed("auto");
    expect(readPersistedThemeMode()).toBe("auto");
  });

  it("returns null for unknown values and corrupt stores (callers keep their default)", () => {
    seed("solarized");
    expect(readPersistedThemeMode()).toBeNull();
    window.localStorage.setItem("lumen.settings.overrides.v1", "{corrupt");
    expect(readPersistedThemeMode()).toBeNull();
  });
});

describe("readPersistedAccent — the bootstrap read (w26.f08 round 2)", () => {
  const seed = (value: unknown): void =>
    window.localStorage.setItem(
      "lumen.settings.overrides.v1",
      JSON.stringify({ "global.accent-color": value }),
    );

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.unstubAllGlobals();
    delete document.documentElement.dataset.theme;
    delete document.documentElement.dataset.accent;
  });

  it("reads the generated accents through the same persistence seam", () => {
    expect(readPersistedAccent()).toBeNull(); // clean store — never guesses
    seed("violet");
    expect(readPersistedAccent()).toBe("violet");
    seed("gray");
    expect(readPersistedAccent()).toBe("gray");
  });

  it("returns null for values outside the generated 9 and corrupt stores", () => {
    seed("magenta");
    expect(readPersistedAccent()).toBeNull();
    window.localStorage.setItem("lumen.settings.overrides.v1", "{corrupt");
    expect(readPersistedAccent()).toBeNull();
  });
});

describe("useSettingsThemeSync (interface.theme → ThemeProvider → <html>)", () => {
  it("applies the schema default (dark) to the html attribute on mount", () => {
    mount(<Harness mode="light" />);
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("flips data-theme on <html> when the setting draft changes (live preview)", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-light"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
  });

  it("flips back when the draft reverts to the persisted value", async () => {
    mount(<Harness mode="light" revertButton />);
    fireEvent.click(screen.getByTestId("set-light"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    fireEvent.click(screen.getByTestId("revert"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("dark"));
  });

  it("keeps data-accent at the orange default when the accent draft is untouched", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-light"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    expect(document.documentElement.dataset.accent).toBe("orange");
  });

  it("flips data-accent on <html> when the accent draft changes (live preview, w26.f08 round 2)", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-accent-violet"));
    await waitFor(() => expect(document.documentElement.dataset.accent).toBe("violet"));
  });

  it("ignores accent drafts outside the generated 9 — never throws through setAccent", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-accent-rogue"));
    // The theme draft never changed; the rogue accent is silently dropped.
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.dataset.accent).toBe("orange");
  });

  it("ignores non-theme values (never guesses a mode)", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-rogue"));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("forwards a SYSTEM (auto) draft — the provider resolves it against the OS preference (e19-F5)", async () => {
    // jsdom has no matchMedia — stub a LIGHT OS preference (the unit-test
    // stand-in for Playwright's emulateMedia colorScheme probe).
    const query = {
      matches: true,
      addEventListener: () => {},
      removeEventListener: () => {},
    };
    vi.stubGlobal("matchMedia", vi.fn().mockReturnValue(query));
    mount(<Harness mode="auto" />);
    fireEvent.click(screen.getByTestId("set-auto"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    vi.unstubAllGlobals();
    delete document.documentElement.dataset.theme;
  });
});
