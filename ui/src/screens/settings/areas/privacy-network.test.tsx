// Card 4.E4 — Privacy & Network group tests: the Network area is an honest
// empty state (no schema entries for proxies), while the privacy half renders
// the frame's History + Network sections (Settings - privacy.svg §C3) with the
// Clear-history action row (g11-P1-1) and the telemetry rows relocated off the
// screen (g11-P2-1: ui=false, CLI/config-only).

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { NetworkArea } from "./privacy-network";
import { SETTING_PRIVACY_CRASH_REPORTS, SETTING_PRIVACY_TELEMETRY } from "../generated/schema";
import { SETTINGS_ENTRIES } from "../generated/schema";
import type { SettingEntry } from "../generated/schema";

// w26.gfix-B (g11-P2-1): the telemetry rows are real schema keys that left
// the UI registry (ui=false) — membership in SETTINGS_ENTRIES is the proof.
const uiKeys = new Set(SETTINGS_ENTRIES.map((e) => e.key));
const isScreenRow = (entry: SettingEntry): boolean => uiKeys.has(entry.key);

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

describe("Privacy half — the frame's History + Network sections (Settings - privacy.svg §C3)", () => {
  it("schema truth: the telemetry rows stay CLI/config-only (ui=false — the privacy frame carries none)", () => {
    expect(SETTING_PRIVACY_TELEMETRY.defaultValue).toBe(false);
    expect(SETTING_PRIVACY_TELEMETRY.defaultRef).toBe("telemetry.enabled");
    // w26.gfix-B (g11-P2-1): the four telemetry rows left the SCREEN with the
    // re-seed — still real schema keys (CLI surface), never deleted silently.
    expect(SETTING_PRIVACY_TELEMETRY.screen).toBe("privacy");
    expect(isScreenRow(SETTING_PRIVACY_TELEMETRY)).toBe(false);
    expect(isScreenRow(SETTING_PRIVACY_CRASH_REPORTS)).toBe(false);
  });

  it("renders the frame's History rows at /settings/privacy — none of the telemetry rows (g11-P2-1)", () => {
    renderRouted("privacy");
    // Frame History: the 3 schema rows in their frame states.
    expect(screen.getByRole("switch", { name: "Save recently played items" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("switch", { name: "Remember the playback position" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("textbox", { name: "Never remember items matching" })).toHaveValue("");
    // Frame History row 4: the Clear-history ACTION (g11-P1-1) — disabled with
    // the honest reason (no history store exists yet), never dead-but-alive.
    expect(screen.getByText("Clear history now")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Clear/ })).toBeDisabled();
    // The relocated telemetry rows render NOTHING here (ui=false).
    expect(screen.queryByText("Share anonymous usage data")).toBeNull();
    expect(screen.queryByRole("button", { name: "Crash reports" })).toBeNull();
    expect(document.querySelector('[data-setting="privacy.telemetry"]')).toBeNull();
    expect(document.querySelector('[data-setting="privacy.crash-reports"]')).toBeNull();
  });

  it("renders the frame's Network section (last.fm rows) verbatim", () => {
    renderRouted("privacy");
    expect(screen.getByRole("switch", { name: "Fetch titles and cover art from the internet" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    expect(screen.getByRole("switch", { name: "Submit played tracks to last.fm" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    expect(screen.getByRole("textbox", { name: "Last.fm username" })).toHaveValue("");
    expect(screen.getByRole("textbox", { name: "Last.fm password" })).toHaveValue("");
    expect(screen.getByRole("textbox", { name: "Last.fm submission URL" })).toHaveValue("");
  });

  it("url rows carry NO Browse companion — the privacy frame's Last.fm inputs are plain wells (w26.gfix-B)", () => {
    renderRouted("privacy");
    // The privacy frame (§C3) shows the three Last.fm inputs as plain 318×34
    // wells; only PATH rows carry the §C0 208+Browse variant (global.log-file,
    // interface.skin-file, recording.directory — all frame-verified).
    expect(screen.queryByRole("button", { name: "Browse" })).toBeNull();
    expect(document.querySelector('[data-setting="privacy.lastfm-submission-url"]')?.getAttribute("data-kind")).toBe(
      "text",
    );
  });
});
