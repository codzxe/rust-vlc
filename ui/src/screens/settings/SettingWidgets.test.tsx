// w26.k12 — SettingWidgets row-control tests (j12 P2-4, the only shell-side
// gap the review carried): the Startup volume Values-1 control is a real
// WAI-ARIA radiogroup — k01's shared roving keyboard grammar
// (useRadiogroupKeyboard: four arrows, wrap-around) on the group host +
// roving tabindex (only the checked radio is a tab stop). The two buttons
// used to be two independent tab stops with no arrow keys at all.

import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import { StartupVolumeField } from "./SettingWidgets";
import { SettingsProvider, useSettings } from "./SettingsProvider";
import {
  SETTING_AUDIO_STARTUP_VOLUME,
  SETTING_AUDIO_STARTUP_VOLUME_MODE,
} from "./generated/schema";

const mount = (ui: ReactElement) => render(<SettingsProvider>{ui}</SettingsProvider>);

/** Reads the mode enum the radiogroup commits through the settings store. */
let latestMode = "";
function ModeProbe() {
  const { value } = useSettings();
  latestMode = String(value(SETTING_AUDIO_STARTUP_VOLUME_MODE.key));
  return null;
}

describe("StartupVolumeField — the Values-1 radiogroup (w26.k12, j12 P2-4)", () => {
  it("renders two radios with roving tabindex — only the checked one is a tab stop", () => {
    mount(
      <>
        <StartupVolumeField entry={SETTING_AUDIO_STARTUP_VOLUME} />
        <ModeProbe />
      </>,
    );
    const group = screen.getByRole("radiogroup", { name: "Startup volume" });
    expect(group).toBeInTheDocument();
    const radios = screen.getAllByRole("radio");
    expect(radios).toHaveLength(2);
    // Default mode "remember-last": radio 1 checked + tabbable, radio 2 not.
    expect(radios[0]).toHaveAccessibleName("Remember last volume");
    expect(radios[0]).toHaveAttribute("aria-checked", "true");
    expect(radios[0]).toHaveAttribute("tabindex", "0");
    expect(radios[1]).toHaveAccessibleName("Always starts at:");
    expect(radios[1]).toHaveAttribute("aria-checked", "false");
    expect(radios[1]).toHaveAttribute("tabindex", "-1");
    // The nested percent input is inert while its row is unselected.
    expect(screen.getByRole("textbox", { name: "Startup volume" })).toBeDisabled();
  });

  it("arrow keys move the selection (wrap-around) and commit the mode key", () => {
    mount(
      <>
        <StartupVolumeField entry={SETTING_AUDIO_STARTUP_VOLUME} />
        <ModeProbe />
      </>,
    );
    expect(latestMode).toBe("remember-last"); // schema default
    const group = screen.getByRole("radiogroup", { name: "Startup volume" });
    // Down: remember-last → always; the store commit + roving tabindex move.
    fireEvent.keyDown(group, { key: "ArrowDown" });
    const radios = screen.getAllByRole("radio");
    expect(radios[1]).toHaveAttribute("aria-checked", "true");
    expect(radios[1]).toHaveAttribute("tabindex", "0");
    expect(radios[0]).toHaveAttribute("tabindex", "-1");
    expect(latestMode).toBe("always");
    // Down again wraps to the first option.
    fireEvent.keyDown(group, { key: "ArrowDown" });
    expect(screen.getAllByRole("radio")[0]).toHaveAttribute("aria-checked", "true");
    expect(latestMode).toBe("remember-last");
    // Left/Right work too; Right moves forward (wrap).
    fireEvent.keyDown(group, { key: "ArrowRight" });
    expect(latestMode).toBe("always");
    fireEvent.keyDown(group, { key: "ArrowLeft" });
    expect(latestMode).toBe("remember-last");
    // Left from the FIRST option wraps BACKWARD to the LAST ("always") —
    // wrap-around works in both directions (k01's shared grammar).
    fireEvent.keyDown(group, { key: "ArrowLeft" });
    expect(latestMode).toBe("always");
  });

  it("clicking a radio selects it (the pre-existing pointer path stays)", () => {
    mount(<StartupVolumeField entry={SETTING_AUDIO_STARTUP_VOLUME} />);
    fireEvent.click(screen.getByRole("radio", { name: "Always starts at:" }));
    expect(screen.getByRole("radio", { name: "Always starts at:" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    // The nested input becomes live (enabled) with its row selected.
    expect(screen.getByRole("textbox", { name: "Startup volume" })).toBeEnabled();
  });
});
