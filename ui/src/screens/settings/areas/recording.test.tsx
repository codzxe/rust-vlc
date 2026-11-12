// w26.b07 — Recording area tests (the b02 sibling contract's recording
// half): the embedded generated screen (the seeded directory + format rows'
// only on-route render path), the F2 feature gate (rows hidden + inert
// while `recording` is off), and the honest-surface rule for the frame's
// schema-less Casting/Streaming rows.

import { fireEvent, render, screen } from "@testing-library/react";
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

describe("RecordingArea — F2 feature gate + the staged §C12 rows (w26.i15)", () => {
  it("keeps the gated rows hidden + inert while `recording` is off (Phase 7, rule F2)", () => {
    const { container } = mountArea();
    expect(RECORDING_ENTRIES.every((entry) => !entry.featureExposed)).toBe(true);
    expect(container.querySelector('[data-setting="recording.directory"]')).toBeNull();
    expect(container.querySelector('[data-setting="recording.format"]')).toBeNull();
    // The staged §C12 rows render instead of an empty-area note — the frame's
    // 10 rows, visible, honest about the seam.
    expect(container.querySelector('[data-testid="settings-area-empty"]')).toBeNull();
    expect(container.querySelectorAll("[data-staged-row]")).toHaveLength(10);
  });

  it("renders the §C12 Casting/Streaming rows STAGED inert, frame-verbatim (never live bindings)", () => {
    const { container } = mountArea();
    // Recording (§C12 y83): the path row + Browse companion + the ON switch.
    expect(screen.getByText("Save recordings to")).toBeInTheDocument();
    expect(screen.getByDisplayValue("~/Videos")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Browse" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Record without re-encoding when possible" })).toBeDisabled();
    // Casting (§C12 y232) + Streaming (§C12 y379), values verbatim.
    for (const label of [
      "Discover cast devices",
      "Cast quality",
      "Play locally while streaming",
      "Include audio",
      "Include video",
      "Include subtitles", // the frame's [sic lowercase in layer] reads as the layer's typo; sibling rows capitalize
      "Encoding preset",
      "Tuning",
    ]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByText("Automatic")).toBeInTheDocument();
    expect(screen.getByText("Medium")).toBeInTheDocument();
    expect(screen.getByText("None")).toBeInTheDocument();
    // Every staged row: the honest tooltip + its seam marker (the Recording
    // rows await the feature gate; Casting/Streaming the schema seed).
    for (const row of container.querySelectorAll("[data-staged-row]")) {
      expect(row.getAttribute("title")).toContain("Staged");
    }
    expect(container.querySelectorAll("[data-setting], [data-setting-capability]")).toHaveLength(0);
  });

  it("carries the frame's ⓘ census (4/10 rows), operable — never the plain rows", () => {
    const { container } = mountArea();
    // SVG circle census (Settings - recording.svg): 4 r7.5 circles —
    // directory, cast-quality, encoding-preset, tuning. The frame's other
    // six rows carry none; controls + extensions frames carry zero total.
    const dots = container.querySelectorAll(".settings-field__info");
    expect(dots).toHaveLength(4);
    const withInfo = new Set(
      [...dots].map((d) => d.closest(".settings-field")?.getAttribute("data-staged-row")),
    );
    expect(withInfo).toEqual(
      new Set(["recording.directory", "recording.cast-quality", "recording.encoding-preset", "recording.tuning"]),
    );
    fireEvent.click(dots[0]!);
    expect(dots[0]!.getAttribute("aria-expanded")).toBe("true");
    expect(container.querySelector(".settings-field__hinttext")?.textContent).toContain(
      "recording ships",
    );
    fireEvent.click(dots[0]!);
    expect(container.querySelector(".settings-field__hinttext")).toBeNull();
  });
});
