// w26.f02 — SettingField row anatomy tests (e06 F1/F2/F7/F11, e08 F2/F10):
// enum rows are CLOSED dropdowns (never stacked radio groups), every row
// type renders the visible left label, and the ⓘ hint affordance exists
// ONLY where a hint text resolves (honest absence otherwise — the schema
// seed carries no hint text today).
// w26.i12 — the enum label fallback (never a blank option/trigger, h09 #1)
// and the frame-exact ⓘ glyph grammar (h08 #3 / h09 #2 / h10 #1).

import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import { SettingField, SettingInfoDot } from "./SettingField";
import { SettingsProvider, useSettings } from "./SettingsProvider";
import {
  SETTING_CONTROLS_SKIP_VERY_SHORT,
  SETTING_INTERFACE_RESIZE_TO_FIT_VIDEO,
  SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES,
  SETTING_PRIVACY_CRASH_REPORTS,
  SETTING_VIDEO_ASPECT_RATIO,
} from "./generated/schema";

const mount = (ui: ReactElement) => render(<SettingsProvider>{ui}</SettingsProvider>);

describe("SettingField — enum rows are closed dropdowns (e06/e08 F1)", () => {
  it("renders a closed combobox trigger with the selected value, no stacked radio group", () => {
    // w26.gfix-B: privacy.crash-reports left the ui registry with the re-seed
    // (g11-P2-1: the privacy frame carries no telemetry rows) — a live enum
    // row carries the closed-dropdown proof (frame value "Never").
    mount(<SettingField entry={SETTING_INTERFACE_RESIZE_TO_FIT_VIDEO} />);
    const trigger = screen.getByRole("button", { name: "Resize window to fit video" });
    expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(trigger).toHaveTextContent("Never"); // frame C2: "Resize window to fit video" → "Never"
    expect(
      document.querySelector('[data-setting="interface.resize-to-fit-video"] .lumen-radiogroup'),
    ).toBeNull();
  });

  it("opens the listbox, selects an option and marks the row dirty", () => {
    mount(<SettingField entry={SETTING_INTERFACE_RESIZE_TO_FIT_VIDEO} />);
    fireEvent.click(screen.getByRole("button", { name: "Resize window to fit video" }));
    const list = screen.getByRole("listbox", { name: "Resize window to fit video" });
    expect(list).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Never", selected: true })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("option", { name: "Always" }));
    expect(screen.queryByRole("listbox")).toBeNull(); // closed after selection
    expect(screen.getByRole("button", { name: "Resize window to fit video" })).toHaveTextContent("Always");
    expect(document.querySelector('[data-setting="interface.resize-to-fit-video"]')).toHaveAttribute(
      "data-dirty",
    );
  });

  it("falls back to the RAW enum value when an option msgid is unregistered (w26.i12, h09 #1)", () => {
    // The video aspect/crop enums carried 10 msgids the catalog had not
    // registered — they rendered BLANK options, and picking one left the
    // trigger empty (the h09 repro). The lane seam (w26.i14) now registers
    // the real labels; the raw-value fallback below stays as the grammar's
    // floor — a catalog gap can never blank an option or trigger again.
    const entry = {
      ...SETTING_VIDEO_ASPECT_RATIO,
      key: "video.aspect-ratio-fallback-probe",
      enumeration: [
        { value: "automatic", msgid: "settings.value.video-aspect-ratio.automatic" },
        { value: "21:9", msgid: "settings.value.video-aspect-ratio.21:9" }, // registered nowhere
      ],
    };
    mount(<SettingField entry={entry} />);
    const trigger = screen.getByRole("button", { name: "Aspect ratio" });
    fireEvent.click(trigger);
    // Every option carries an accessible name — no blank listbox rows.
    expect(screen.getByRole("option", { name: "21:9" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("option", { name: "21:9" }));
    // The trigger shows the picked value — never blank (the h09 repro).
    expect(trigger).toHaveTextContent("21:9");
    expect(trigger.textContent ?? "").not.toBe("");
  });

  it("renders the frame's video aspect labels end-to-end and keeps the trigger after pick (h09 repro)", () => {
    mount(<SettingField entry={SETTING_VIDEO_ASPECT_RATIO} />);
    const trigger = screen.getByRole("button", { name: "Aspect ratio" });
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("option", { name: "16:9" }));
    expect(trigger).toHaveTextContent("16:9");
    expect(trigger.textContent ?? "").not.toBe("");
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

  it("duration (duration-ms) rows render the frame's formatted duration (g13-P2-7)", () => {
    // w26.gfix-B: the controls frame shows "3 s" in the well — a text input
    // with a numeric parse (native number controls show raw ms and no unit).
    let latest = -1;
    function Probe() {
      const { value } = useSettings();
      latest = Number(value(SETTING_CONTROLS_SKIP_VERY_SHORT.key));
      return null;
    }
    mount(
      <>
        <SettingField entry={SETTING_CONTROLS_SKIP_VERY_SHORT} />
        <Probe />
      </>,
    );
    const field = document.querySelector('[data-setting="controls.skip-very-short"]');
    expect(field?.querySelector(".settings-field__lead .settings-field__label")).toHaveTextContent(
      "Very short skip",
    );
    const input = screen.getByRole("textbox", { name: "Very short skip" }) as HTMLInputElement;
    expect(input).toHaveValue("3 s"); // frame C8: seeded 3000 ms → "3 s"
    fireEvent.change(input, { target: { value: "10 s" } });
    expect(input).toHaveValue("10 s");
    // The human duration parses back to milliseconds in the SAME store.
    expect(latest).toBe(10000);
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

  it("carries the frame-exact ⓘ glyph: 18px info sheet = r7.5 circle + 1.125px ring (w26.i12)", () => {
    // Settings - *.svg "Ellipse 9": circle r7.5 stroke #404040 sw1.125 +
    // i-glyph sw1.5. The shared "info" sheet (24-unit viewBox, circle r10,
    // sw2 stem/dot) at size 18 lands exactly on those frame metrics — the
    // glyph the 26 missing frame ⓘ affordances inherit (h08 #3 / h09 #2 /
    // h10 #1).
    const { container } = render(<SettingInfoDot note="Staged" label="More info" />);
    const glyph = container.querySelector(".settings-field__info svg.lumen-icon");
    expect(glyph).not.toBeNull();
    expect(glyph).toHaveAttribute("width", "18");
    expect(glyph).toHaveAttribute("height", "18");
    expect(glyph).toHaveAttribute("aria-hidden", "true"); // the button carries the name
    // Uncontrolled use (the areas' staged rows): no aria-expanded promise.
    expect(screen.getByRole("button", { name: "More info" })).not.toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });
});
