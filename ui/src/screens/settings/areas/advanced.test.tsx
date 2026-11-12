// Card 4.E4 — Advanced area tests: every advanced-flagged schema entry folds
// into one screen, collapsed by default, expands on demand, and nothing that
// is not advanced-flagged (or not on a UI screen) is rendered there.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { AdvancedArea, advancedEntries } from "./advanced";
import { SETTINGS_ENTRIES } from "../generated/schema";

describe("advancedEntries — schema-driven membership", () => {
  it("collects every advanced-flagged entry on UI screens (and no engine ui=false entry)", () => {
    const keys = advancedEntries().map((e) => e.key);
    // w26.gfix-B: the ui=false window metrics left the registry with the
    // re-seed; the live advanced set carries the proof (global-1 rows,
    // the gated policy enum, the performance resources + Developer rows).
    expect(keys).toContain("global.show-minor-warnings");
    expect(keys).toContain("global.collect-playback-statistics");
    expect(keys).toContain("playback.hw-decode-policy");
    expect(keys).toContain("performance.cache-demux-mib");
    expect(keys).toContain("performance.max-open-sessions");
    expect(keys).toContain("performance.encoder"); // Developer rows (g14-P2-3: flat fold)
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

  it("is redirected from /settings/advanced to its nearest home (performance) via the registry (w26.b02 fold)", async () => {
    render(
      <ClientProvider transport={new MockTransport()}>
        <MemoryRouter initialEntries={["/settings/advanced"]}>
          <Routes>
            <Route path="/settings/:area" element={<SettingsScreen />} />
          </Routes>
        </MemoryRouter>
      </ClientProvider>,
    );
    // The Advanced AREA dissolved into the per-area in-flow disclosure
    // (w26.b02); the slug redirects to performance — its nearest home —
    // and the redirected screen's top-bar title announces it.
    expect(await screen.findByText("Performance")).toBeInTheDocument();
  });
});
