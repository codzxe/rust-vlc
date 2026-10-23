import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { SettingsProvider, useSettings } from "../../SettingsProvider";
import { memorySettingsPersistence } from "../../persistence";
import { SETTING_GLOBAL_LANGUAGE } from "../../generated/schema";
import { LocalePicker } from "./LocalePicker";
import { AVAILABLE_LOCALES, REFERENCE_LOCALE, localeLabel } from "./locales";
import { useMsg } from "../../../../i18n/messages";

// Card 4.E2 language area: the locale picker renders ONLY real locales (the
// localization crate's locales/ dir) and binds to the global.language key.

const mount = () =>
  render(
    <SettingsProvider persistence={memorySettingsPersistence}>
      <LocalePicker entry={SETTING_GLOBAL_LANGUAGE} />
    </SettingsProvider>,
  );

describe("LocalePicker (global.language)", () => {
  it("renders exactly the real available locales (localization crate data)", () => {
    mount();
    // One radio per available locale; en-US is the only shipped catalog today.
    const radios = screen.getAllByRole("radio");
    expect(radios).toHaveLength(AVAILABLE_LOCALES.length);
    expect(screen.getByRole("radio", { name: /English \(United States\)/i })).toBeTruthy();
  });

  it("does not offer schema values that lack a shipped catalog (never invented)", () => {
    const { container } = mount();
    const offered = [...container.querySelectorAll("input[type=radio]")].map((i) => (i as HTMLInputElement).value);
    expect(offered).toEqual(["en-US"]);
    expect(offered).not.toContain("xx-XX");
  });

  it("binds to the global.language setting key (change → draft value + dirty)", async () => {
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
    // proven by the radio carrying the schema key's current value.
    const radio = screen.getByRole("radio", { name: /English \(United States\)/i }) as HTMLInputElement;
    expect(radio.value).toBe("en-US");
    fireEvent.click(radio);
    expect(latest).toBe("en-US");
  });

  it("labels the reference locale through the i18n catalog", () => {
    const m = useMsg();
    expect(localeLabel(REFERENCE_LOCALE, m)).toBe("English (United States)");
    expect(localeLabel("fr-FR", m)).toBe("fr-FR"); // no catalog copy → raw tag
  });
});
