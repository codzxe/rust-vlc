import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { SettingsProvider, useSettings } from "../../SettingsProvider";
import { memorySettingsPersistence } from "../../persistence";
import { SETTING_INTERFACE_THEME } from "../../generated/schema";
import { ThemeModePicker } from "./ThemeModePicker";
import { ThemeProvider } from "../../../../theme/ThemeProvider";

// w26.f03 (e05 F4) — the Theme row control is the frame's 3-cell SEGMENTED
// control (Settings - global.svg "Theme": 320×36 track, active cell
// accent-tinted), not a stacked radio list. Light · Dark · System; the
// "applies instantly" hint paragraph is gone (no frame equivalent).

const mount = () =>
  render(
    <ThemeProvider>
      <SettingsProvider persistence={memorySettingsPersistence}>
        <ThemeModePicker entry={SETTING_INTERFACE_THEME} />
      </SettingsProvider>
    </ThemeProvider>,
  );

describe("ThemeModePicker (interface.theme — segmented anatomy, w26.f03)", () => {
  it("renders exactly the three frame cells in a radiogroup", () => {
    mount();
    const group = screen.getByRole("radiogroup", { name: "Theme" });
    expect(group).toHaveClass("lumen-segmented"); // the SegmentedControl grammar
    expect(screen.getByRole("radio", { name: "Light" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Dark" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "System" })).toBeInTheDocument();
    expect(screen.getAllByRole("radio")).toHaveLength(3);
  });

  it("drops the old 'applies instantly' hint paragraph (no frame equivalent)", () => {
    mount();
    expect(screen.queryByText("Theme changes apply instantly for preview.")).toBeNull();
  });

  it("binds cell clicks to the settings draft and previews live", async () => {
    let latest = "";
    function Probe() {
      const { value } = useSettings();
      latest = String(value(SETTING_INTERFACE_THEME.key));
      return null;
    }
    render(
      <ThemeProvider>
        <SettingsProvider persistence={memorySettingsPersistence}>
          <ThemeModePicker entry={SETTING_INTERFACE_THEME} />
          <Probe />
        </SettingsProvider>
      </ThemeProvider>,
    );
    const light = screen.getByRole("radio", { name: "Light" });
    fireEvent.click(light);
    expect(latest).toBe("light");
    await (() => expect(document.documentElement.dataset.theme).toBe("light"))();
    // The System cell writes the ThemeProvider's own mode-setting id.
    fireEvent.click(screen.getByRole("radio", { name: "System" }));
    expect(latest).toBe("auto");
    // Dark round-trips back.
    fireEvent.click(screen.getByRole("radio", { name: "Dark" }));
    expect(latest).toBe("dark");
  });

  it("keeps arrow-key roving inside the segmented control (component contract)", () => {
    mount();
    const dark = screen.getByRole("radio", { name: "Dark" });
    fireEvent.keyDown(dark, { key: "ArrowRight" });
    // Schema order light/dark + system: ArrowRight from Dark lands on System.
    const system = screen.getByRole("radio", { name: "System" });
    expect(system).toHaveAttribute("aria-checked", "true");
  });
});
