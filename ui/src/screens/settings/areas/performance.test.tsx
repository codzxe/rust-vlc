// w26.k13 — Performance area tests (j13 P1-1, the regression test the review
// asked for: the performance area had NO dedicated test file, which is exactly
// why its ⓘ census gap went undetected). Pins:
//   • the frame's ⓘ census — 10 of 15 rows (circle extraction over
//     `design/figma/Settings - performance.svg`: Ellipse 9..9_10 — the four
//     Decoding rows, Process priority, and the five advanced Developer rows),
//     with the honest absences reproduced (the two Smooth-Playback toggles
//     carry no circle; the three resources rows sit below the frame fold);
//   • every schema row still renders bound to its key through the framework
//     (search + feature gates + dirty/restart stay SettingField-driven);
//   • the Advanced Options disclosure carries the flat advanced rows
//     (the SettingsSections grammar the generated screen used — no regression
//     from the hand render).

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ReactNode } from "react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { PerformanceArea } from "./performance";
import { SETTINGS_ENTRIES } from "../generated/schema";

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

const PERFORMANCE_ENTRIES = SETTINGS_ENTRIES.filter((entry) => entry.screen === "performance");

describe("PerformanceArea — the frame's sections from the seed", () => {
  it("renders every non-advanced schema row bound to its key, in its section", () => {
    render(
      <SettingsProvider>
        <PerformanceArea />
      </SettingsProvider>,
    );
    expect(
      document.querySelector('[data-settings-area="performance"]'),
    ).not.toBeNull();
    for (const entry of PERFORMANCE_ENTRIES.filter((e) => !e.advanced)) {
      const field = document.querySelector(`[data-setting="${entry.key}"]`);
      expect(field, `performance entry ${entry.key} must render bound to its schema key`).not.toBeNull();
    }
    // The advanced rows (resources + Developer) render only behind the bar.
    for (const entry of PERFORMANCE_ENTRIES.filter((e) => e.advanced)) {
      expect(document.querySelector(`[data-setting="${entry.key}"]`)).toBeNull();
    }
  });

  it("frame row values render through the framework (Decoding + Smooth Playback)", () => {
    render(
      <SettingsProvider>
        <PerformanceArea />
      </SettingsProvider>,
    );
    // Decoding — the four enum/int rows (schema defaults, Settings - performance.svg).
    expect(screen.getByRole("button", { name: "Hardware-accelerated decoding" })).toHaveTextContent(
      "Automatic",
    );
    expect(screen.getByRole("button", { name: "Video post-processing" })).toHaveTextContent("Off");
    expect(screen.getByRole("button", { name: "Skip the H.264 loop filter" })).toHaveTextContent("Never");
    expect(
      (screen.getByRole("spinbutton", { name: "Decoding threads" }) as HTMLInputElement).value,
    ).toBe("0");
    // Smooth Playback — the two toggles + the priority input.
    expect(screen.getByRole("switch", { name: "Drop frames when the system can't keep up" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("switch", { name: "Give VLC real-time priority" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    expect(
      (screen.getByRole("spinbutton", { name: "Process priority" }) as HTMLInputElement).value,
    ).toBe("0");
  });

  it("the Advanced Options bar carries the flat advanced rows (resources + Developer, schema order)", () => {
    render(
      <SettingsProvider>
        <PerformanceArea />
      </SettingsProvider>,
    );
    const bar = screen.getByRole("button", { name: "Advanced Options" });
    expect(bar).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(bar);
    expect(bar).toHaveAttribute("aria-expanded", "true");
    // The flat advanced list, schema order across sections: the 3 resources
    // rows then the 5 Developer rows (the generated screen's grammar).
    const advancedKeys = PERFORMANCE_ENTRIES.filter((e) => e.advanced).map((e) => e.key);
    const rendered = [...document.querySelectorAll(".settings-advanced__rows [data-setting]")].map(
      (el) => el.getAttribute("data-setting"),
    );
    expect(rendered).toEqual(advancedKeys);
  });
});

describe("PerformanceArea — the frame's ⓘ census (w26.k13, j13 P1-1)", () => {
  it("wires exactly the 10 frame circles — collapsed 5, +5 Developer rows behind the bar", () => {
    // Circle extraction over Settings - performance.svg: Ellipse 9_4/9_3/9_2/9
    // on the Decoding rows (cy 132/178/224/271), Ellipse 9_5 on Process
    // priority (cy 462), Ellipse 9_10..9_6 on the Developer rows
    // (cy 668/714/760/807/855). THE regression pin: the pure-generated screen
    // wired ZERO of them.
    routed("/settings/performance", routes);
    const info = '[data-settings-area="performance"] .settings-field__info';
    expect(document.querySelectorAll(info)).toHaveLength(5);
    fireEvent.click(screen.getByRole("button", { name: "Advanced Options" }));
    expect(document.querySelectorAll(info)).toHaveLength(10);
  });

  it("reproduces the honest absences: the two Smooth-Playback toggles and the resources rows are bare", () => {
    routed("/settings/performance", routes);
    fireEvent.click(screen.getByRole("button", { name: "Advanced Options" }));
    for (const key of [
      "performance.drop-frames", // frame: the y359 toggle row carries no circle
      "performance.realtime-priority", // frame: the y401 toggle row carries no circle
      "performance.cache-demux-mib", // below the frame's exported fold — no ⓘ claim
      "performance.cache-network-mib",
      "performance.max-open-sessions",
    ]) {
      expect(
        document.querySelector(`[data-setting="${key}"] .settings-field__info`),
        `${key} carries no frame circle`,
      ).toBeNull();
    }
  });

  it("the ⓘ are operable — the reveal carries the hint copy (the shared SettingInfoDot grammar)", () => {
    routed("/settings/performance", routes);
    const dot = document.querySelector(
      '[data-setting="performance.hw-decode"] .settings-field__info',
    ) as HTMLButtonElement;
    expect(dot).toHaveAttribute("aria-expanded", "false");
    expect(dot.getAttribute("title")).toContain("graphics hardware");
    fireEvent.click(dot);
    expect(dot).toHaveAttribute("aria-expanded", "true");
    expect(
      document.querySelector('[data-setting="performance.hw-decode"] .settings-field__hinttext')
        ?.textContent,
    ).toContain("graphics hardware");
  });
});
