import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ThemeProvider } from "./ThemeProvider";
import { useSettingsThemeSync } from "./settingsThemeSync";
import { SettingsProvider, useSettings } from "../screens/settings/SettingsProvider";
import { memorySettingsPersistence } from "../screens/settings/persistence";
import { SETTING_INTERFACE_THEME } from "../screens/settings/generated/schema";

// Card 4.E2 appearance wiring: setting change → ThemeProvider state → the
// data-theme attribute on <html> (doc 08 §3.3 attribute-only theming).

/** Mounts the sync seam plus a radio that writes interface.theme for real. */
function Harness({ mode, revertButton = false }: { mode: string; revertButton?: boolean }) {
  const { setValue, revert } = useSettings();
  return (
    <>
      <Sync />
      <input type="radio" data-testid={`set-${mode}`} onChange={() => setValue(SETTING_INTERFACE_THEME.key, mode)} />
      <input type="radio" data-testid="set-rogue" onChange={() => setValue(SETTING_INTERFACE_THEME.key, "solarized")} />
      {revertButton ? <button type="button" data-testid="revert" onClick={() => revert()} /> : null}
    </>
  );
}

function Sync() {
  useSettingsThemeSync();
  return null;
}

const mount = (ui: React.ReactNode) =>
  render(
    <ThemeProvider>
      <SettingsProvider persistence={memorySettingsPersistence}>{ui}</SettingsProvider>
    </ThemeProvider>,
  );

describe("useSettingsThemeSync (interface.theme → ThemeProvider → <html>)", () => {
  it("applies the schema default (dark) to the html attribute on mount", () => {
    mount(<Harness mode="light" />);
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("flips data-theme on <html> when the setting draft changes (live preview)", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-light"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
  });

  it("flips back when the draft reverts to the persisted value", async () => {
    mount(<Harness mode="light" revertButton />);
    fireEvent.click(screen.getByTestId("set-light"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    fireEvent.click(screen.getByTestId("revert"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("dark"));
  });

  it("never touches data-accent (no interface.accent schema key — recorded gap)", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-light"));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    // 4.A2's default stands until the schema owner adds the accent key.
    expect(document.documentElement.dataset.accent).toBe("orange");
  });

  it("ignores non-theme values (never guesses a mode)", async () => {
    mount(<Harness mode="light" />);
    fireEvent.click(screen.getByTestId("set-rogue"));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});
