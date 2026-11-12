import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { ThemeProvider } from "../../../theme/ThemeProvider";
import { UI_PACKAGE_VERSION } from "../../../build-info";
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
    expect(trigger?.textContent ?? "").toMatch(/English/i);
  });

  it("renders the global area frame-true: the Advanced bar is LAST; About rides behind the disclosure (w26.i13, h08 #1/#2)", () => {
    routed("/settings/global", routes);
    // Settings - global.svg: the column ends at the Advanced Options bar —
    // collapsed state carries NO About section in the flow (the former
    // beforeAdvanced placement displaced the bar 141px and clipped all four
    // expanded rows below the window fold).
    const bar = screen.getByRole("button", { name: "Advanced Options" });
    expect(document.querySelector('[data-testid="about-area"]')).toBeNull();
    const column = document.querySelector('[data-settings-area="global"]');
    expect(column?.lastElementChild?.classList.contains("settings-advanced")).toBe(true);
    // Expand: the four advanced rows render BELOW the bar (global-1), then
    // the About fold (reachable, out of the frame-true column).
    fireEvent.click(bar);
    expect(bar).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Show minor warnings")).toBeInTheDocument();
    expect(screen.getByText("Collect playback statistics")).toBeInTheDocument();
    expect(document.querySelector('[data-testid="about-area"]')).toBeTruthy();
    expect(screen.getByTestId("about-version").textContent).toBe(UI_PACKAGE_VERSION);
  });

  it("wires the frame's ⓘ hint affordances on global — 6 + the 4 advanced rows (w26.i13, h08 #3)", () => {
    // Circle extraction over Settings - global.svg: Startup×2 + Diagnostics×4;
    // the Appearance rows carry NO circle. Settings - global-1.svg adds the 4
    // advanced rows (cy 724/768/814/858).
    routed("/settings/global", routes);
    const info = '[data-settings-area="global"] .settings-field__info';
    expect(document.querySelectorAll(info)).toHaveLength(6);
    fireEvent.click(screen.getByRole("button", { name: "Advanced Options" }));
    expect(document.querySelectorAll(info)).toHaveLength(10);
  });

  it("wires the frame's ⓘ on interface — 11 of 13 rows, the two honest absences (w26.i13, h08 #3)", () => {
    routed("/settings/interface", routes);
    expect(
      document.querySelectorAll('[data-settings-area="interface"] .settings-field__info'),
    ).toHaveLength(11);
    expect(
      document.querySelector('[data-setting="interface.remember-window-geometry"] .settings-field__info'),
    ).toBeNull(); // frame cy 367 carries no circle
    expect(
      document.querySelector('[data-setting="interface.hide-controls-after"] .settings-field__info'),
    ).toBeNull(); // the "Hide after" label carries none (the cy-783 ⓘ is the row below)
  });

  it("wires the frame's ⓘ on privacy — every row incl. the Clear action (w26.i13, h08 #3)", () => {
    routed("/settings/privacy", routes);
    expect(
      document.querySelectorAll('[data-settings-area="privacy"] .settings-field__info'),
    ).toHaveLength(9); // 8 schema rows + the hand Clear-history action row
  });

  it("wires the frame's ⓘ on playback — 12 of 13 rows, the slider row honestly bare (w26.i14, h09 #2)", () => {
    // Circle extraction over Settings - playback.svg: every row carries the
    // r7.5 circle EXCEPT "Default playback speed" (the slider row, no circle
    // at its ~y227 band; the frame's 12 circles sit at cy 132/176/285/388/
    // 434/480/526/631/675/717/761/864).
    routed("/settings/playback", routes);
    expect(
      document.querySelectorAll('[data-settings-area="playback"] .settings-field__info'),
    ).toHaveLength(12);
    expect(
      document.querySelector('[data-setting="playback.default-rate"] .settings-field__info'),
    ).toBeNull(); // the slider row carries no circle
  });

  it("wires the frame's ⓘ on video — Output/Deinterlacing/Framing, the Picture rows honestly bare (w26.i14, h09 #2)", () => {
    // Circle extraction over Settings - video.svg: 4 Output toggles +
    // 2 Deinterlacing + 2 Framing dropdowns (cy 130/172/214/256/670/716/821/
    // 867); the five Picture sliders and the Reset action carry none.
    routed("/settings/video", routes);
    expect(
      document.querySelectorAll('[data-settings-area="video"] .settings-field__info'),
    ).toHaveLength(8);
    for (const key of ["video.hue", "video.brightness", "video.contrast", "video.saturation", "video.gamma"]) {
      expect(
        document.querySelector(`[data-setting="${key}"] .settings-field__info`),
        `${key} carries no frame circle`,
      ).toBeNull();
    }
    expect(document.querySelector('[data-video-action="reset-picture"] .settings-field__info')).toBeNull();
  });

  it("wires the frame's ⓘ on audio — 10 of 11 rows, the Maximum-volume slider honestly bare (w26.i14, h10 #1)", () => {
    // Circle extraction over Settings - audio.svg: Output×3 (incl. the
    // capability-backed hand row — its ⓘ is unconditional, the reason is its
    // tooltip while not ready) · Startup volume · Volume step · the 5 Loudness
    // rows; the Maximum-volume slider carries none.
    routed("/settings/audio", routes);
    expect(
      document.querySelectorAll('[data-settings-area="audio"] .settings-field__info'),
    ).toHaveLength(10);
    expect(
      document.querySelector('[data-setting="audio.maximum-volume"] .settings-field__info'),
    ).toBeNull(); // the slider row carries no circle
  });

  it("renders the video area with the codecs fold (w26 merge: codecs→video)", () => {
    routed("/settings/video", routes);
    expect(document.querySelector('[data-settings-area="video"]')).toBeTruthy();
    // w26.i14: the codecs rows sit behind the in-flow disclosure (collapsed
    // by default) — open it, then the merged area renders.
    const bar = screen.getByRole("button", { name: "Advanced Options" });
    fireEvent.click(bar);
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
