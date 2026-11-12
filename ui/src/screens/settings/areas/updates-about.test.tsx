// Card 4.E4 — Updates & About area tests: the About panel shows REAL version
// data imported from the package manifest (never a hardcoded literal).
// w26.gfix-B: the update-check row MOVED to the global screen's Startup
// section (g10-P2-1 — key global.update-checks, frame copy "Check for
// updates automatically"); this fold renders the About panel only.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { UI_PACKAGE_NAME, UI_PACKAGE_VERSION } from "../../../build-info";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { AboutArea } from "./updates-about";
import { SETTING_GLOBAL_UPDATE_CHECKS } from "../generated/schema";

// The build-injected package identity (vite.config.ts reads ui/package.json
// and `define`s it; src/build-info.ts re-exports it). The test proves the
// About panel displays exactly the injected build data — the component
// itself carries no version literal to drift.

const renderRouted = (area: string) =>
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[`/settings/${area}`]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>
    </ClientProvider>,
  );

describe("AboutArea — About panel shows real package data", () => {
  it("shows the version injected from the package manifest (never hardcoded)", () => {
    render(
      <SettingsProvider>
        <AboutArea />
      </SettingsProvider>,
    );
    expect(screen.getByTestId("about-version").textContent).toBe(UI_PACKAGE_VERSION);
    expect(screen.getByTestId("about-package-name").textContent).toBe(UI_PACKAGE_NAME);
    expect(UI_PACKAGE_VERSION).toMatch(/^\d+\.\d+\.\d+/); // real semver, not a placeholder
  });

  it("is routed at /settings/about via the areas registry (about→global fold)", () => {
    // w26.i13 (h08 #2): the About fold renders BEHIND the global screen's
    // Advanced Options disclosure (neither global frame shows an About
    // section; the in-flow placement displaced the bar 141px) — the redirect
    // lands on /settings/global, the fold opens with the disclosure.
    renderRouted("about");
    expect(document.querySelector('[data-settings-area="about"]')).toBeNull(); // collapsed: frame-true column
    fireEvent.click(screen.getByRole("button", { name: "Advanced Options" }));
    expect(document.querySelector('[data-settings-area="about"]')).not.toBeNull();
    expect(screen.getByTestId("about-version").textContent).toBe(UI_PACKAGE_VERSION);
  });

  it("renders the About rows in the settings row grammar (g10-P2-3 restyle)", () => {
    const { container } = render(
      <SettingsProvider>
        <AboutArea />
      </SettingsProvider>,
    );
    const rows = container.querySelectorAll("[data-about-row]");
    expect(rows).toHaveLength(2);
    for (const row of rows) {
      expect(row.querySelector(".settings-field__lead .settings-field__label")).toBeTruthy();
      expect(row.querySelector(".settings-about__value")).toBeTruthy();
    }
  });
});

describe("update checks — moved to the global Startup section (w26.gfix-B)", () => {
  it("schema truth: global.update-checks defaults false via telemetry.update_checks (opt-in)", () => {
    expect(SETTING_GLOBAL_UPDATE_CHECKS.key).toBe("global.update-checks");
    expect(SETTING_GLOBAL_UPDATE_CHECKS.defaultValue).toBe(false);
    expect(SETTING_GLOBAL_UPDATE_CHECKS.defaultRef).toBe("telemetry.update_checks");
    expect(SETTING_GLOBAL_UPDATE_CHECKS.featureGate).toBe(""); // ungated: follows schema, not features
  });

  it("renders the frame-verbatim row on /settings/global, OFF by default — the fold no longer duplicates it", () => {
    const { container } = renderRouted("global");
    const field = container.querySelector('[data-setting="global.update-checks"]');
    expect(field).not.toBeNull();
    expect(screen.getByText("Check for updates automatically")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Check for updates automatically" }).getAttribute("aria-checked")).toBe(
      "false",
    );
    // The Updates fold is GONE from the global screen (g10-P2-1): exactly one
    // update-check row renders.
    expect(container.querySelectorAll('[data-setting="global.update-checks"]')).toHaveLength(1);
    expect(screen.queryByText("Check for updates")).toBeNull();
  });

  it("renders no update action the command catalog does not define (config-never-invents)", () => {
    render(
      <SettingsProvider>
        <AboutArea />
      </SettingsProvider>,
    );
    expect(screen.queryByRole("button", { name: /check for updates/i })).toBeNull();
  });
});
