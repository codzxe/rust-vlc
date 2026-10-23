// Card 4.E4 — Advanced area tests: every advanced-flagged schema entry folds
// into one screen, collapsed by default, expands on demand, and nothing that
// is not advanced-flagged (or not on a UI screen) is rendered there.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { AdvancedArea, advancedEntries } from "./advanced";
import { SETTINGS_ENTRIES } from "../generated/schema";

describe("advancedEntries — schema-driven membership", () => {
  it("collects every advanced-flagged entry on UI screens (and no engine ui=false entry)", () => {
    const keys = advancedEntries().map((e) => e.key);
    expect(keys).toContain("interface.window-min-width");
    expect(keys).toContain("interface.window-min-height");
    expect(keys).toContain("playback.hw-decode-policy");
    expect(keys).toContain("performance.cache-demux-mib");
    expect(keys).toContain("performance.max-open-sessions");
    expect(keys.every((k) => !k.startsWith("engine."))).toBe(true);
    for (const entry of SETTINGS_ENTRIES.filter((e) => e.advanced && !e.screen.startsWith("engine"))) {
      expect(keys).toContain(entry.key);
    }
  });
});

describe("AdvancedArea — fold/expand (E1 advanced-badge mechanism)", () => {
  it("is collapsed by default: the fold body is hidden, no fields visible", () => {
    render(
      <SettingsProvider>
        <AdvancedArea />
      </SettingsProvider>,
    );
    const fold = screen.getByTestId("advanced-fold");
    expect(fold.getAttribute("aria-expanded")).toBe("false");
    const body = screen.getByTestId("advanced-fold-body");
    expect(body).toHaveAttribute("hidden");
  });

  it("expands on click and renders only advanced-flagged schema keys", () => {
    const { container } = render(
      <SettingsProvider>
        <AdvancedArea />
      </SettingsProvider>,
    );
    fireEvent.click(screen.getByTestId("advanced-fold"));
    expect(screen.getByTestId("advanced-fold").getAttribute("aria-expanded")).toBe("true");
    const advancedKeys = new Set(advancedEntries().map((e) => e.key));
    const rendered = [...container.querySelectorAll("[data-setting]")].map((el) =>
      el.getAttribute("data-setting"),
    );
    expect(rendered.length).toBeGreaterThan(0);
    for (const key of rendered) {
      expect(advancedKeys.has(key as string), `${key} must be advanced-flagged to render here`).toBe(true);
    }
  });

  it("keeps the F2-gated hw-decode policy hidden inside the fold (doc 05 §6.4)", () => {
    render(
      <SettingsProvider>
        <AdvancedArea />
      </SettingsProvider>,
    );
    fireEvent.click(screen.getByTestId("advanced-fold"));
    // SettingField hides non-exposed entries; the honest disabled-with-reason
    // presentation lives on the codecs area instead (see codecs.test.tsx).
    expect(document.querySelector('[data-setting="playback.hw-decode-policy"]')).toBeNull();
  });

  it("is routed at /settings/advanced via the areas registry", () => {
    render(
      <MemoryRouter initialEntries={["/settings/advanced"]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>,
    );
    expect(document.querySelector('[data-settings-area="advanced"]')).not.toBeNull();
  });
});
