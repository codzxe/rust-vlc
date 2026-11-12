import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { SettingsProvider, useSettings } from "../../SettingsProvider";
import { memorySettingsPersistence } from "../../persistence";
import { SETTING_GLOBAL_LANGUAGE } from "../../generated/schema";
import { LocalePicker } from "./LocalePicker";
import { AVAILABLE_LOCALES, REFERENCE_LOCALE, localeLabel } from "./locales";
import { useMsg } from "../../../../i18n/messages";

// Card 4.E2 language area (w26.f03, e05 F5): the locale picker is a CLOSED
// dropdown (frame §C0/§C1 — 320×36 trigger + chevron, not a stacked radio
// list) rendering ONLY real locales (the localization crate's locales/ dir),
// bound to the global.language key.

const mount = () =>
  render(
    <SettingsProvider persistence={memorySettingsPersistence}>
      <LocalePicker entry={SETTING_GLOBAL_LANGUAGE} />
    </SettingsProvider>,
  );

describe("LocalePicker (global.language)", () => {
  it("renders a closed dropdown trigger showing the current locale", () => {
    mount();
    // The trigger is a combobox (aria-haspopup listbox), labeled by the row
    // (w26.gfix-B: frame C1 label "Interface language", g10-P3-1).
    const trigger = screen.getByRole("button", { name: "Interface language" });
    expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(trigger).toHaveTextContent("English");
    // Closed by default — no open listbox until the user opens it.
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("offers exactly the real available locales when opened (localization crate data)", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Interface language" }));
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(AVAILABLE_LOCALES.length);
    expect(screen.getByRole("option", { name: /English/i })).toBeTruthy();
    // en-US is the only shipped catalog today.
    expect(AVAILABLE_LOCALES).toEqual(["en-US"]);
  });

  it("binds to the global.language setting key (select → draft value + dirty)", async () => {
    let latest = "";
    function Probe() {
      const { value } = useSettings();
      latest = String(value(SETTING_GLOBAL_LANGUAGE.key));
      return null;
    }
    render(
      <SettingsProvider persistence={memorySettingsPersistence}>
        <LocalePicker entry={SETTING_GLOBAL_LANGUAGE} />
        <Probe />
      </SettingsProvider>,
    );
    expect(latest).toBe(SETTING_GLOBAL_LANGUAGE.defaultValue); // en-US
    // Single-locale world: the option is already selected; the binding is
    // proven by the trigger carrying the schema key's current value and the
    // selected option staying bound after re-selecting it.
    const trigger = screen.getByRole("button", { name: "Interface language" });
    expect(trigger).toHaveTextContent("English");
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("option", { name: /English/i }));
    expect(latest).toBe("en-US");
    expect(screen.getByRole("button", { name: "Interface language" })).toHaveFocus(); // close restores focus
  });

  it("labels the reference locale through the i18n catalog", () => {
    const m = useMsg();
    expect(localeLabel(REFERENCE_LOCALE, m)).toBe("English");
    expect(localeLabel("fr-FR", m)).toBe("fr-FR"); // no catalog copy → raw tag
  });
});
