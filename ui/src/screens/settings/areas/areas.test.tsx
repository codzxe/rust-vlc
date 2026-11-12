import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { ThemeProvider } from "../../../theme/ThemeProvider";
import {
  SETTINGS_AREA_IDS,
  SETTINGS_AREA_REDIRECTS,
  SETTINGS_AREA_TITLES,
  SETTINGS_SCREENS,
  SPECIAL_FIELD_WIDGETS,
} from "./index";
import { SETTINGS_ENTRIES } from "../generated/schema";
import { SETTING_INTERFACE_THEME, SETTING_GLOBAL_LANGUAGE } from "../generated/schema";
import { AVAILABLE_LOCALES } from "./language/locales";
import { isMessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b02";

// Card 4.E2 AC: +SB and every option bound to a REAL setting key.
// w26.b02: the registry is EXACTLY the 13 Figma areas (analysis D3) with the
// Lead's merges; the retired hotkeys/codecs/about/advanced/effects slugs are
// redirect data, not areas.

// Issue #3 wiring: SettingsScreen consumes the client context (the desktop
// config seam) — every routed mount supplies the honest MockTransport.
const routed = (initial: string, children: ReactNode) =>
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[initial]}>{children}</MemoryRouter>
    </ClientProvider>,
  );

const routes = (
  <Routes>
    <Route path="/settings/:area" element={<SettingsScreen />} />
  </Routes>
);

describe("areas layer — the 13-area registry (w26.b02)", () => {
  it("is exactly the 13 Figma areas in sidebar order (App 4 · Media 5 · System 4)", () => {
    expect(SETTINGS_AREA_IDS).toEqual([
      "global", "interface", "privacy", "controls",
      "playback", "video", "audio", "subtitles", "library",
      "network", "performance", "recording", "extensions",
    ]);
    for (const id of SETTINGS_AREA_IDS) {
      expect(SETTINGS_SCREENS[id], `area ${id} must have a screen`).toBeTruthy();
      expect(SETTINGS_AREA_TITLES[id], `area ${id} must have a title key`).toBeTruthy();
    }
  });

  it("every special widget key IS a real schemas/settings.toml key (AC)", () => {
    const schemaKeys = new Set(SETTINGS_ENTRIES.map((e) => e.key));
    for (const key of Object.keys(SPECIAL_FIELD_WIDGETS)) {
      expect(schemaKeys.has(key), `special widget key "${key}" must exist in schemas/settings.toml`).toBe(true);
    }
  });

  it("area titles resolve via the catalog or the lane keys (§4.5 pre-registration seam)", () => {
    for (const id of SETTINGS_AREA_IDS) {
      const key = SETTINGS_AREA_TITLES[id];
      expect(
        isMessageKey(key) || key in laneKeys,
        `area title "${key}" must be registered or carried by ui/src/i18n/keys/w26.b02.ts`,
      ).toBe(true);
    }
  });

  it("maps every retired slug to its merged home and off the 13-area registry", () => {
    expect(SETTINGS_AREA_REDIRECTS).toEqual({
      hotkeys: "controls",
      codecs: "video",
      about: "global",
      advanced: "performance",
      effects: "audio",
    });
    for (const [retired, home] of Object.entries(SETTINGS_AREA_REDIRECTS)) {
      expect(SETTINGS_AREA_IDS).not.toContain(retired);
      expect(SETTINGS_AREA_IDS).toContain(home);
    }
  });

  it("renders the Interface screen with every schema entry bound to its key", () => {
    routed("/settings/interface", routes);
    for (const entry of SETTINGS_ENTRIES.filter((e) => e.screen === "interface" && !e.advanced)) {
      const field = document.querySelector(`[data-setting="${entry.key}"]`);
      expect(field, `interface entry ${entry.key} must render bound to its schema key`).toBeTruthy();
    }
    // w26.f03 (e05 F7): the interface frame carries NO "Advanced Options"
    // bar — advanced-flagged rows render FLAT in their sections (the
    // disclosure belongs to the frames that show it: global, performance…).
    const advanced = SETTINGS_ENTRIES.filter((e) => e.screen === "interface" && e.advanced);
    for (const entry of advanced) {
      const field = document.querySelector(`[data-setting="${entry.key}"]`);
      expect(field, `interface advanced entry ${entry.key} renders flat (no disclosure)`).toBeTruthy();
    }
  });

  it("renders the Language screen (global area) with the special locale picker", () => {
    routed("/settings/global", routes);
    const field = document.querySelector(`[data-setting="${SETTING_GLOBAL_LANGUAGE.key}"]`);
    expect(field).toBeTruthy();
    // w26.f03 (e05 F5): the locale control is a closed Dropdown (the frame's
    // 320×36 grammar), not a radio stack — the trigger's accessible name
    // carries the current locale.
    const trigger = field?.querySelector<HTMLButtonElement>('[aria-haspopup="listbox"]');
    expect(trigger, "the locale picker is a dropdown trigger").toBeTruthy();
    expect(trigger?.textContent ?? "").toMatch(/English \(United States\)/i);
  });

  it("renders the global area with the about fold (w26 merge: about→global)", () => {
    routed("/settings/global", routes);
    expect(document.querySelector('[data-testid="about-area"]')).toBeTruthy();
  });

  it("renders the video area with the codecs fold (w26 merge: codecs→video)", () => {
    routed("/settings/video", routes);
    expect(document.querySelector('[data-settings-area="video"]')).toBeTruthy();
    expect(document.querySelector('[data-settings-area="codecs"]')).toBeTruthy();
  });

  it("renders the audio area with the effects fold (w26 merge: effects→audio)", () => {
    routed("/settings/audio", routes);
    expect(document.querySelector('[data-settings-area="audio"]')).toBeTruthy();
    expect(document.querySelector('[data-settings-area="effects"]')).toBeTruthy();
  });

  it("navigates between merged areas from the settings nav", async () => {
    routed("/settings/interface", routes);
    fireEvent.click(screen.getByRole("button", { name: "Privacy" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Privacy" })).toHaveAttribute("aria-current", "page"),
    );
  });

  it("the special theme picker replaces the interface.theme control inside the shell (live wiring)", async () => {
    // w26.gfix-B (g10-P1-2): the theme row MOVED to the global screen's
    // Appearance section (the interface frame carries no theme control) — the
    // live wiring proof follows it there.
    render(
      <ThemeProvider>
        <ClientProvider transport={new MockTransport()}>
          <MemoryRouter initialEntries={["/settings/global"]}>{routes}</MemoryRouter>
        </ClientProvider>
      </ThemeProvider>,
    );
    const field = document.querySelector(`[data-setting="${SETTING_INTERFACE_THEME.key}"]`);
    expect(field).toBeTruthy(); // special ThemeModePicker inside the E1 wrapper
    const light = screen.getByRole("radio", { name: "Light" });
    fireEvent.click(light);
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
  });
});

describe("language area — real locales only", () => {
  it("offers only locales with a shipped catalog (locales/ dir)", () => {
    expect(AVAILABLE_LOCALES).toContain("en-US");
    const enumValues = SETTING_GLOBAL_LANGUAGE.enumeration.map((o) => o.value);
    for (const locale of AVAILABLE_LOCALES) {
      expect(enumValues, `available locale ${locale} must be bindable via the schema enumeration`).toContain(locale);
    }
  });
});

// NOTE (w26.b02): this file imports ./index, which imports the sibling area
// components ./controls, ./recording, ./extensions, ./library under the
// wave-B fixed names. Until lanes b07/b08 land those files, module
// resolution fails HERE (and in SettingsScreen.test.tsx) — expected
// parallel-lane gap, documented in docs/waves/w26/logs/w26.b02.md.
