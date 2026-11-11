// Card 4.E4 — Input & Codecs area tests: the hw-decode policy control is
// bound to the REAL schema key, rendered disabled-with-reason per ADR-001's
// honest status + product.toml F2 exposure, and the cache controls are bound
// to their real schema keys (options-bound-to-schema proof).

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { CodecsArea, HwDecodePolicyControl } from "./codecs";
import {
  SETTING_PERFORMANCE_CACHE_DEMUX_MIB,
  SETTING_PERFORMANCE_CACHE_NETWORK_MIB,
  SETTING_PLAYBACK_HW_DECODE_POLICY,
} from "../generated/schema";
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

describe("CodecsArea — routing + schema-bound options", () => {
  it("is routed at /settings/codecs via the areas registry", () => {
    renderRouted("codecs");
    expect(document.querySelector('[data-settings-area="codecs"]')).not.toBeNull();
  });

  it("renders the cache controls bound to their real schema keys", () => {
    render(
      <SettingsProvider>
        <CodecsArea />
      </SettingsProvider>,
    );
    expect(document.querySelector(`[data-setting="${SETTING_PERFORMANCE_CACHE_DEMUX_MIB.key}"]`)).not.toBeNull();
    expect(document.querySelector(`[data-setting="${SETTING_PERFORMANCE_CACHE_NETWORK_MIB.key}"]`)).not.toBeNull();
    // Both are advanced-flagged in the schema. Since w26.b02 the E1 advanced
    // presentation is the area-level in-flow disclosure (one grouped badge /
    // fold, not one badge per row) — at least one Advanced affordance shows.
    expect(screen.getAllByText("Advanced").length).toBeGreaterThanOrEqual(1);
  });

  it("binds the hw-decode control to playback.hw-decode-policy with the full schema enumeration", () => {
    render(
      <SettingsProvider>
        <HwDecodePolicyControl />
      </SettingsProvider>,
    );
    expect(document.querySelector('[data-setting="playback.hw-decode-policy"]')).not.toBeNull();
    for (const option of SETTING_PLAYBACK_HW_DECODE_POLICY.enumeration) {
      expect(screen.getByRole("radio", { name: new RegExp(option.value.replace("-", " "), "i") })).toBeInTheDocument();
    }
  });
});

describe("hw-decode — ADR-001 honest status + F2 feature gating", () => {
  it("schema truth: gated on hw-decode, NOT exposed (product.toml F2 keeps it false until Phase 7)", () => {
    expect(SETTING_PLAYBACK_HW_DECODE_POLICY.featureGate).toBe("hw-decode");
    expect(SETTING_PLAYBACK_HW_DECODE_POLICY.featureExposed).toBe(false);
  });

  it("renders disabled with the honest reason (never a dead-but-alive control)", () => {
    render(
      <SettingsProvider>
        <HwDecodePolicyControl />
      </SettingsProvider>,
    );
    for (const radio of screen.getAllByRole("radio")) {
      expect(radio).toBeDisabled();
    }
    expect(document.querySelector('[data-gate-reason="hw-decode"]')).not.toBeNull();
    expect(screen.getByText(/ADR-001/i)).toBeInTheDocument();
    expect(screen.getByText(/F2/)).toBeInTheDocument();
  });

  it("stays inert: clicking a policy radio cannot change the value while gated", () => {
    render(
      <SettingsProvider>
        <HwDecodePolicyControl />
      </SettingsProvider>,
    );
    const software = screen.getByRole("radio", { name: /software only/i }) as HTMLInputElement;
    expect(software.checked).toBe(true);
    fireEvent.click(screen.getByRole("radio", { name: /hardware only/i }));
    expect(software.checked).toBe(true);
  });

  it("shows the shipped product default (software-only, product.toml §2.8) as selected", () => {
    render(
      <SettingsProvider>
        <HwDecodePolicyControl />
      </SettingsProvider>,
    );
    expect(SETTING_PLAYBACK_HW_DECODE_POLICY.defaultValue).toBe("software-only");
    expect((screen.getByRole("radio", { name: /software only/i }) as HTMLInputElement).checked).toBe(true);
  });

  it("renders no codec-family options the schema does not define (config-never-invents)", () => {
    const { container } = render(
      <SettingsProvider>
        <CodecsArea />
      </SettingsProvider>,
    );
    const rendered = [...container.querySelectorAll("[data-setting]")].map((el) =>
      el.getAttribute("data-setting"),
    );
    const schemaKeys = new Set(SETTINGS_ENTRIES.map((e) => e.key));
    for (const key of rendered) {
      expect(schemaKeys.has(key as string), `rendered key ${key} must exist in schemas/settings.toml`).toBe(true);
    }
  });
});
