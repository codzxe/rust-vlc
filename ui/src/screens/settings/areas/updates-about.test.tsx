// Card 4.E4 — Updates & About area tests: the About panel shows REAL version
// data imported from the package manifest (never a hardcoded literal), and
// the update-check toggle is bound to its real schema key, default OFF.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { UI_PACKAGE_NAME, UI_PACKAGE_VERSION } from "../../../build-info";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { AboutArea } from "./updates-about";
import { SETTING_PRIVACY_UPDATE_CHECKS } from "../generated/schema";

// The build-injected package identity (vite.config.ts reads ui/package.json
// and `define`s it; src/build-info.ts re-exports it). The test proves the
// About panel displays exactly the injected build data — the component
// itself carries no version literal to drift.

const renderRouted = (area: string) =>
  render(
    <MemoryRouter initialEntries={[`/settings/${area}`]}>
      <Routes>
        <Route path="/settings/:area" element={<SettingsScreen />} />
      </Routes>
    </MemoryRouter>,
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

  it("is routed at /settings/about via the areas registry", () => {
    renderRouted("about");
    expect(document.querySelector('[data-settings-area="about"]')).not.toBeNull();
    expect(screen.getByTestId("about-version").textContent).toBe(UI_PACKAGE_VERSION);
  });
});

describe("AboutArea — update checks (schema + features)", () => {
  it("schema truth: privacy.update-checks defaults false via telemetry.update_checks (opt-in)", () => {
    expect(SETTING_PRIVACY_UPDATE_CHECKS.defaultValue).toBe(false);
    expect(SETTING_PRIVACY_UPDATE_CHECKS.defaultRef).toBe("telemetry.update_checks");
    expect(SETTING_PRIVACY_UPDATE_CHECKS.featureGate).toBe(""); // ungated: follows schema, not features
  });

  it("renders the update-check toggle bound to its key, OFF by default", () => {
    render(
      <SettingsProvider>
        <AboutArea />
      </SettingsProvider>,
    );
    expect(document.querySelector('[data-setting="privacy.update-checks"]')).not.toBeNull();
    expect(screen.getByRole("switch", { name: /update/i }).getAttribute("aria-checked")).toBe("false");
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
