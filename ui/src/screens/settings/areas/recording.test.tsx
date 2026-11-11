// w26.b07 — Recording area tests (the b02 sibling contract's recording
// half): the embedded generated screen (the seeded directory + format rows'
// only on-route render path), the F2 feature gate (rows hidden + inert
// while `recording` is off), and the honest-surface rule for the frame's
// schema-less Casting/Streaming rows.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { RecordingArea } from "./recording";
import { GENERATED_SCREENS } from "../generated/Screens";
import { SETTINGS_ENTRIES } from "../generated/schema";

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

const mountArea = () =>
  render(
    <SettingsProvider>
      <RecordingArea />
    </SettingsProvider>,
  );

const RECORDING_ENTRIES = SETTINGS_ENTRIES.filter((entry) => entry.screen === "recording");

describe("RecordingArea — routing + the b02 sibling contract", () => {
  it("is routed at /settings/recording under the area registry", () => {
    renderRouted("recording");
    expect(document.querySelector('[data-settings-area="recording"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: "Recording" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("embeds GENERATED_SCREENS.recording (the seeded rows' only on-route render path)", () => {
    const { container } = mountArea();
    expect(GENERATED_SCREENS.recording).toBeTruthy();
    // The generated screen's own root (data-settings-area, no testid)
    // renders nested inside this area's wrapper — the sibling contract's
    // "must embed" clause made structural: the wrapper carries the testid,
    // the embedded screen does not.
    const embedded = container.querySelector<HTMLElement>(
      '.settings-sections[data-settings-area="recording"]:not([data-testid])',
    );
    expect(embedded).not.toBeNull();
    expect(container.querySelector('[data-testid="recording-area"]')).toContainElement(embedded);
  });

  it("schema truth: the b02 seed carries the directory + format rows, gated on `recording`", () => {
    const keys = RECORDING_ENTRIES.map((entry) => entry.key);
    expect(keys).toEqual(["recording.directory", "recording.format"]);
    for (const entry of RECORDING_ENTRIES) {
      expect(entry.featureGate).toBe("recording");
    }
  });
});

describe("RecordingArea — F2 feature gate + honest surface", () => {
  it("keeps the gated rows hidden + inert while `recording` is off (Phase 7, rule F2)", () => {
    const { container } = mountArea();
    expect(RECORDING_ENTRIES.every((entry) => !entry.featureExposed)).toBe(true);
    expect(container.querySelector('[data-setting="recording.directory"]')).toBeNull();
    expect(container.querySelector('[data-setting="recording.format"]')).toBeNull();
    // The area shows the honest empty state instead of dead controls.
    expect(container.querySelector('[data-testid="settings-area-empty"]')).not.toBeNull();
  });

  it("renders NO dead Casting/Streaming controls (no schema keys, no catalog commands)", () => {
    mountArea();
    const frameLabels = [
      "Discover cast devices",
      "Cast quality",
      "Play locally while streaming",
      "Include audio",
      "Include video",
      "Include subtitles",
      "Encoding preset",
      "Tuning",
    ];
    for (const label of frameLabels) {
      expect(
        screen.queryByText(label),
        `frame row "${label}" has no schema key → renders nothing`,
      ).toBeNull();
    }
  });
});
