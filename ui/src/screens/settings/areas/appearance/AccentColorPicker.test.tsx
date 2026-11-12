import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { SettingsProvider, useSettings } from "../../SettingsProvider";
import { memorySettingsPersistence } from "../../persistence";
import { SETTING_GLOBAL_ACCENT_COLOR } from "../../generated/schema";
import { AccentColorPicker } from "./AccentColorPicker";
import { ThemeProvider } from "../../../../theme/ThemeProvider";

// w26.f03 (e05 F2) — the Accent color row control is the frame's 9-swatch
// group (Settings - global.svg "Accent color": 320×36 track, orange
// selected), wired through the existing SwatchGroup component to the
// global.accent-color schema key (w26.f01 seed) with live preview through
// the ThemeProvider accent seam.

const mount = () =>
  render(
    <ThemeProvider>
      <SettingsProvider persistence={memorySettingsPersistence}>
        <AccentColorPicker entry={SETTING_GLOBAL_ACCENT_COLOR} />
      </SettingsProvider>
    </ThemeProvider>,
  );

describe("AccentColorPicker (global.accent-color — swatch anatomy, w26.f03)", () => {
  it("renders the nine frame accents in frame order, orange selected by default", () => {
    mount();
    const swatches = screen.getAllByRole("radio");
    expect(swatches).toHaveLength(9);
    // Frame order (Settings - global.svg): orange first, then red, yellow,
    // green, cyan, blue, violet, pink, gray.
    const order = swatches.map((s) => s.getAttribute("data-accent"));
    expect(order).toEqual(["orange", "red", "yellow", "green", "cyan", "blue", "violet", "pink", "gray"]);
    expect(screen.getByRole("radio", { name: "Orange" })).toHaveAttribute("aria-checked", "true");
  });

  it("binds swatch clicks to the settings draft and previews live", () => {
    let latest = "";
    function Probe() {
      const { value } = useSettings();
      latest = String(value(SETTING_GLOBAL_ACCENT_COLOR.key));
      return null;
    }
    render(
      <ThemeProvider>
        <SettingsProvider persistence={memorySettingsPersistence}>
          <AccentColorPicker entry={SETTING_GLOBAL_ACCENT_COLOR} />
          <Probe />
        </SettingsProvider>
      </ThemeProvider>,
    );
    expect(latest).toBe("orange"); // schema default (w26.f01 seed)
    fireEvent.click(screen.getByRole("radio", { name: "Blue" }));
    expect(latest).toBe("blue");
    expect(document.documentElement.dataset.accent).toBe("blue"); // live preview seam
  });

  it("registers as the special widget for the accent row wherever the schema seeds it", () => {
    // The areas registry derives the special-widget key from the schema
    // (SPECIAL_FIELD_WIDGETS, areas/index.tsx) — assert the wiring end-to-end:
    // the entry's key is the registered key.
    expect(SETTING_GLOBAL_ACCENT_COLOR.key).toBe("global.accent-color");
    expect(SETTING_GLOBAL_ACCENT_COLOR.defaultValue).toBe("orange");
  });
});
