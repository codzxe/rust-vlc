// Card 4.E4 — Privacy & Network group tests: the Network area is an honest
// empty state (no schema entries for proxies), while the privacy half renders
// via E1's generated screen with product.toml §2.6 opt-in defaults (OFF).

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { NetworkArea } from "./privacy-network";
import { SETTING_PRIVACY_CRASH_REPORTS, SETTING_PRIVACY_TELEMETRY } from "../generated/schema";

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

describe("NetworkArea — honest empty surface (proxies are schema-absent)", () => {
  it("renders the honest empty state and zero controls", () => {
    const { container } = render(<NetworkArea />);
    expect(container.querySelector('[data-settings-area="network"]')).not.toBeNull();
    expect(screen.getByTestId("settings-area-empty")).toBeVisible();
    expect(container.querySelectorAll("[data-setting], [data-setting-capability]")).toHaveLength(0);
  });

  it("invents no proxy options the schema does not define", () => {
    render(<NetworkArea />);
    expect(screen.queryByText(/proxy|socks/i)).toBeNull();
  });

  it("is routed at /settings/network via the areas registry", () => {
    renderRouted("network");
    expect(document.querySelector('[data-settings-area="network"]')).not.toBeNull();
  });
});

describe("Privacy half — telemetry-off defaults on the generated screen", () => {
  it("schema truth: telemetry defaults to false via product.toml telemetry.enabled (opt-in only)", () => {
    expect(SETTING_PRIVACY_TELEMETRY.defaultValue).toBe(false);
    expect(SETTING_PRIVACY_TELEMETRY.defaultRef).toBe("telemetry.enabled");
  });

  it("renders the telemetry switch OFF (unchecked) at /settings/privacy", () => {
    renderRouted("privacy");
    const toggle = screen.getByRole("switch", { name: "Share anonymous usage data" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(document.querySelector('[data-setting="privacy.telemetry"]')).not.toBeNull();
  });

  it("renders crash-reports at its shipped off/ask/send default (off)", () => {
    expect(SETTING_PRIVACY_CRASH_REPORTS.defaultValue).toBe("off");
    renderRouted("privacy");
    expect((screen.getByRole("radio", { name: "Off" }) as HTMLInputElement).checked).toBe(true);
  });
});
