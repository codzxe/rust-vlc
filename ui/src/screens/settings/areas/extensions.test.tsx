// w26.b08 — Extensions area tests (the b02 sibling contract's extensions
// half): the routed area, the embedded generated screen (the seeded
// load-on-startup row's only on-route render path), the F2 feature gate,
// the hand add-on list + toolbar (honest surface), and the row anatomy.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { ExtensionRow, ExtensionsArea } from "./extensions";
import type { ExtensionInfo } from "./extensions";
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
      <ExtensionsArea />
    </SettingsProvider>,
  );

const EXTENSIONS_ENTRIES = SETTINGS_ENTRIES.filter((entry) => entry.screen === "extensions");

describe("ExtensionsArea — routing + the b02 sibling contract", () => {
  it("is routed at /settings/extensions under the area registry", () => {
    renderRouted("extensions");
    expect(document.querySelector('[data-settings-area="extensions"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: "Extensions" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("embeds GENERATED_SCREENS.extensions (the seeded row's only on-route render path)", () => {
    const { container } = mountArea();
    expect(GENERATED_SCREENS.extensions).toBeTruthy();
    // The generated screen's own root (data-settings-area, no testid)
    // renders nested inside this area's wrapper — the sibling contract's
    // "must embed" clause made structural (the recording-lane assertion
    // pattern): the wrapper carries the testid, the embedded screen does not.
    const embedded = container.querySelector<HTMLElement>(
      '.settings-sections[data-settings-area="extensions"]:not([data-testid])',
    );
    expect(embedded).not.toBeNull();
    expect(container.querySelector('[data-testid="extensions-area"]')).toContainElement(embedded);
  });

  it("schema truth: the b02 seed carries exactly the load-on-startup row, gated on wasm-extensions", () => {
    const keys = EXTENSIONS_ENTRIES.map((entry) => entry.key);
    expect(keys).toEqual(["extensions.load-on-startup"]);
    for (const entry of EXTENSIONS_ENTRIES) {
      expect(entry.featureGate).toBe("wasm-extensions");
    }
  });
});

describe("ExtensionsArea — F2 feature gate", () => {
  it("keeps the seeded row hidden + inert while wasm-extensions is off (Phase 7, rule F2)", () => {
    const { container } = mountArea();
    expect(EXTENSIONS_ENTRIES.every((entry) => !entry.featureExposed)).toBe(true);
    expect(container.querySelector('[data-setting="extensions.load-on-startup"]')).toBeNull();
  });
});

describe("ExtensionsArea — the hand Add-ons surface (list + toolbar)", () => {
  it("renders the section header, the list well and the honest empty state", () => {
    mountArea();
    expect(screen.getByRole("heading", { name: "Add-ons" })).toBeInTheDocument();
    expect(screen.getByTestId("extensions-list")).toBeInTheDocument();
    expect(screen.getByTestId("extensions-list-empty")).toBeInTheDocument();
    expect(screen.getByText("No add-ons installed")).toBeInTheDocument();
  });

  it("renders no fabricated add-on rows (the list data seam is empty)", () => {
    const { container } = mountArea();
    expect(container.querySelectorAll("[data-extension]")).toHaveLength(0);
  });

  it("toolbar: the frame actions render disabled-from-data with the honest reason", () => {
    mountArea();
    for (const name of ["Find more extensions", "Add", "Remove"]) {
      const button = screen.getByRole("button", { name });
      expect(button).toBeDisabled();
      expect(button).toHaveAttribute("aria-describedby", "extensions-gate-note");
    }
    expect(screen.getByText("Not available in this build")).toBeInTheDocument();
  });

  it("toolbar: the search input is a real local filter over the list", () => {
    mountArea();
    const search = screen.getByPlaceholderText("Search");
    expect(search).toHaveValue("");
    fireEvent.change(search, { target: { value: "demo" } });
    expect(search).toHaveValue("demo");
    // The (empty) list filters without crashing — still the honest empty state.
    expect(screen.getByTestId("extensions-list-empty")).toBeInTheDocument();
  });
});

describe("ExtensionRow — the [VERIFY] row anatomy (name + version + form Switch)", () => {
  const fixture: ExtensionInfo = {
    id: "demo-addon",
    name: "Demo add-on",
    version: "1.2.0",
    enabled: true,
  };

  it("renders the add-on name, the tabular version and the ON/OFF Switch", () => {
    render(<ExtensionRow info={fixture} onToggle={() => {}} />);
    expect(screen.getByText("Demo add-on")).toBeInTheDocument();
    expect(screen.getByText("1.2.0")).toBeInTheDocument();
    const toggle = screen.getByRole("switch", { name: "Demo add-on" });
    expect(toggle).toHaveAttribute("aria-checked", "true");
  });

  it("fires the enable toggle through the form API (onChange with the next state)", () => {
    const onToggle = vi.fn();
    render(<ExtensionRow info={fixture} onToggle={onToggle} />);
    fireEvent.click(screen.getByRole("switch", { name: "Demo add-on" }));
    expect(onToggle).toHaveBeenCalledWith(false);
  });
});
