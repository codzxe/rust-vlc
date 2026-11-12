// w26.gfix-B — Input & Codecs area tests (g12-P1-6 rewrite): the hw-decode
// policy control is bound to the REAL schema key, rendered as the settings
// row grammar's CLOSED dropdown (never the retired always-expanded radio
// stack), disabled-with-reason per ADR-001's honest status + product.toml F2
// exposure; the cache controls stay bound to their real schema keys.

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

const mountControl = () =>
  render(
    <SettingsProvider>
      <HwDecodePolicyControl />
    </SettingsProvider>,
  );

describe("CodecsArea — routing + schema-bound options", () => {
  it("is folded under /settings/video via the areas registry (codecs→video)", () => {
    renderRouted("codecs");
    expect(document.querySelector('[data-settings-area="codecs"]')).not.toBeNull();
    expect(document.querySelector('[data-settings-area="video"]')).not.toBeNull();
  });

  it("renders the cache controls bound to their real schema keys", () => {
    render(
      <SettingsProvider>
        <CodecsArea />
      </SettingsProvider>,
    );
    expect(document.querySelector(`[data-setting="${SETTING_PERFORMANCE_CACHE_DEMUX_MIB.key}"]`)).not.toBeNull();
    expect(document.querySelector(`[data-setting="${SETTING_PERFORMANCE_CACHE_NETWORK_MIB.key}"]`)).not.toBeNull();
    // w26.gfix-B (g12-P1-6): the row-level "Advanced" badge is gone with the
    // radio stack — the honest status is the gate note under the policy row.
    expect(screen.queryByText("Advanced")).toBeNull();
  });

  it("binds the hw-decode control to playback.hw-decode-policy with the full schema enumeration", () => {
    mountControl();
    expect(document.querySelector('[data-setting="playback.hw-decode-policy"]')).not.toBeNull();
    const trigger = screen.getByRole("button", { name: "Hardware decode policy" });
    expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
    expect(trigger).toBeDisabled(); // F2-gated — the list never opens
  });
});

describe("hw-decode — ADR-001 honest status + F2 feature gating", () => {
  it("schema truth: gated on hw-decode, NOT exposed (product.toml F2 keeps it false until Phase 7)", () => {
    expect(SETTING_PLAYBACK_HW_DECODE_POLICY.featureGate).toBe("hw-decode");
    expect(SETTING_PLAYBACK_HW_DECODE_POLICY.featureExposed).toBe(false);
  });

  it("renders disabled with the honest reason (never a dead-but-alive control)", () => {
    mountControl();
    const trigger = screen.getByRole("button", { name: "Hardware decode policy" });
    expect(trigger).toBeDisabled();
    expect(document.querySelector('[data-gate-reason="hw-decode"]')).not.toBeNull();
    expect(screen.getByText(/ADR-001/i)).toBeInTheDocument();
    expect(screen.getByText(/F2/)).toBeInTheDocument();
    // The closed trigger keeps the frame's 320×36 shape with the shipped
    // product default selected — never a collapsed stub.
    expect(trigger).toHaveTextContent("Software only");
  });

  it("stays inert: the gated policy cannot be changed while disabled", () => {
    mountControl();
    const trigger = screen.getByRole("button", { name: "Hardware decode policy" });
    fireEvent.click(trigger); // disabled — nothing opens, nothing changes
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(trigger).toHaveTextContent("Software only");
  });

  it("shows the shipped product default (software-only, product.toml §2.8) as selected", () => {
    mountControl();
    expect(SETTING_PLAYBACK_HW_DECODE_POLICY.defaultValue).toBe("software-only");
    expect(screen.getByRole("button", { name: "Hardware decode policy" })).toHaveTextContent("Software only");
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
