import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { SettingsScreen } from "../SettingsScreen";
import { ThemeProvider } from "../../../theme/ThemeProvider";
import { SETTINGS_AREA_IDS, SETTINGS_AREA_TITLES, SETTINGS_SCREENS, SPECIAL_FIELD_WIDGETS } from "./index";
import { SETTINGS_ENTRIES } from "../generated/schema";
import { SETTING_INTERFACE_THEME, SETTING_GLOBAL_LANGUAGE } from "../generated/schema";
import { AVAILABLE_LOCALES } from "./language/locales";
import { useMsg } from "../../../i18n/messages";

// Card 4.E2 AC: +SB and EVERY option bound to a REAL setting key.

describe("areas layer — extension registry", () => {
  it("appends the hotkeys area after the generated schema areas", () => {
    expect(SETTINGS_AREA_IDS).toEqual(["global", "interface", "privacy", "playback", "performance", "hotkeys"]);
    expect(SETTINGS_SCREENS.hotkeys).toBeTruthy();
    expect(useMsg()(SETTINGS_AREA_TITLES.hotkeys)).toBe("Hotkeys");
  });

  it("every special widget key IS a real schemas/settings.toml key (AC)", () => {
    const schemaKeys = new Set(SETTINGS_ENTRIES.map((e) => e.key));
    for (const key of Object.keys(SPECIAL_FIELD_WIDGETS)) {
      expect(schemaKeys.has(key), `special widget key "${key}" must exist in schemas/settings.toml`).toBe(true);
    }
  });

  it("renders the Interface screen with every schema entry bound to its key", () => {
    render(
      <MemoryRouter initialEntries={["/settings/interface"]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>,
    );
    for (const entry of SETTINGS_ENTRIES.filter((e) => e.screen === "interface")) {
      const field = document.querySelector(`[data-setting="${entry.key}"]`);
      expect(field, `interface entry ${entry.key} must render bound to its schema key`).toBeTruthy();
    }
  });

  it("renders the Language screen (global area) with the special locale picker", () => {
    render(
      <MemoryRouter initialEntries={["/settings/global"]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>,
    );
    const field = document.querySelector(`[data-setting="${SETTING_GLOBAL_LANGUAGE.key}"]`);
    expect(field).toBeTruthy();
    expect(screen.getByRole("radio", { name: /English \(United States\)/i })).toBeTruthy();
  });

  it("renders the Hotkeys area with rows from the generated keymap data", () => {
    render(
      <MemoryRouter initialEntries={["/settings/hotkeys"]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByTestId("hotkeys-table")).toBeTruthy();
    expect(document.querySelectorAll(".hotkeys-row").length).toBeGreaterThan(100);
  });

  it("navigates to the hotkeys area from the settings nav", async () => {
    render(
      <MemoryRouter initialEntries={["/settings/interface"]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Hotkeys" }));
    await waitFor(() => expect(screen.getByTestId("hotkeys-table")).toBeTruthy());
  });

  it("the special theme picker replaces the interface.theme control inside the shell (live wiring)", async () => {
    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={["/settings/interface"]}>
          <Routes>
            <Route path="/settings/:area" element={<SettingsScreen />} />
          </Routes>
        </MemoryRouter>
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
