// w26.f02 — SettingField row anatomy tests (e06 F1/F2/F7/F11, e08 F2/F10):
// enum rows are CLOSED dropdowns (never stacked radio groups), every row
// type renders the visible left label, and the ⓘ hint affordance exists
// ONLY where a hint text resolves (honest absence otherwise — the schema
// seed carries no hint text today).

import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import { SettingField } from "./SettingField";
import { SettingsProvider } from "./SettingsProvider";
import {
  SETTING_CONTROLS_SKIP_VERY_SHORT,
  SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES,
  SETTING_PRIVACY_CRASH_REPORTS,
} from "./generated/schema";

const mount = (ui: ReactElement) => render(<SettingsProvider>{ui}</SettingsProvider>);

describe("SettingField — enum rows are closed dropdowns (e06/e08 F1)", () => {
  it("renders a closed combobox trigger with the selected value, no stacked radio group", () => {
    mount(<SettingField entry={SETTING_PRIVACY_CRASH_REPORTS} />);
    const trigger = screen.getByRole("button", { name: "Crash reports" });
    expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(trigger).toHaveTextContent("Off"); // shipped product default
    expect(
      document.querySelector('[data-setting="privacy.crash-reports"] .lumen-radiogroup'),
    ).toBeNull();
  });

  it("opens the listbox, selects an option and marks the row dirty", () => {
    mount(<SettingField entry={SETTING_PRIVACY_CRASH_REPORTS} />);
    fireEvent.click(screen.getByRole("button", { name: "Crash reports" }));
    const list = screen.getByRole("listbox", { name: "Crash reports" });
    expect(list).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Off", selected: true })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("option", { name: "Send" }));
    expect(screen.queryByRole("listbox")).toBeNull(); // closed after selection
    expect(screen.getByRole("button", { name: "Crash reports" })).toHaveTextContent("Send");
    expect(document.querySelector('[data-setting="privacy.crash-reports"]')).toHaveAttribute(
      "data-dirty",
    );
  });
});

describe("SettingField — the row grammar is label-left for every type (e06/e08 F2)", () => {
  it("text (list<string>) rows render the visible lead label beside the control", () => {
    mount(<SettingField entry={SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES} />);
    const field = document.querySelector('[data-setting="playback.preferred-audio-languages"]');
    expect(field?.querySelector(".settings-field__lead .settings-field__label")).toHaveTextContent(
      "Preferred audio languages",
    );
    // The control keeps its own associated (screen-reader-only) label.
    expect(screen.getByRole("textbox", { name: "Preferred audio languages" })).toBeInTheDocument();
  });

  it("number (duration-ms) rows render the visible lead label beside the control", () => {
    mount(<SettingField entry={SETTING_CONTROLS_SKIP_VERY_SHORT} />);
    const field = document.querySelector('[data-setting="controls.skip-very-short"]');
    expect(field?.querySelector(".settings-field__lead .settings-field__label")).toHaveTextContent(
      "Very short skip",
    );
    const input = screen.getByRole("spinbutton", { name: "Very short skip" }) as HTMLInputElement;
    expect(input).toHaveValue(3000); // seeded default, 3 s
  });
});

describe("SettingField — ⓘ hint affordance (e06 F11)", () => {
  it("renders NO ⓘ without a hint — the seed carries none (honest absence)", () => {
    mount(<SettingField entry={SETTING_PRIVACY_CRASH_REPORTS} />);
    expect(screen.queryByRole("button", { name: "More info" })).toBeNull();
  });

  it("renders the ⓘ where a hint exists and reveals the text on demand", () => {
    mount(<SettingField entry={SETTING_PRIVACY_CRASH_REPORTS} hint="settings.field.more-info" />);
    const info = screen.getByRole("button", { name: "More info" });
    expect(info).toHaveAttribute("aria-expanded", "false");
    expect(info).toHaveAttribute("title", "More info");
    expect(screen.queryByText("More info", { selector: ".settings-field__hinttext" })).toBeNull();
    fireEvent.click(info);
    expect(info).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByText("More info", { selector: ".settings-field__hinttext" }),
    ).toBeInTheDocument();
  });
});
