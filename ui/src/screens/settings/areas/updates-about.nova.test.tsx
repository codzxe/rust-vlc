// Card 4.H2 — Rebrand acceptance, About surface (doc 07 §8.3 step 4).
//
// The About panel's ONLY identity source is the build-info seam (card 4.E4:
// vite `define` reads the UI package manifest; src/build-info.ts re-exports
// it; the panel never hardcodes a name). This test proves the propagation
// half of the rebrand runtime plane: with the Nova fixture's identity
// values INJECTED AT THE SEAM (the same values tests/rebrand/nova.toml
// carries — name "Nova Player", display_version "0.2.0-nova"), the panel
// renders Nova and carries zero Lumen identity.
//
// The build-time half (product.toml → package manifest → define) is the
// packaging job's rewrite, recorded honestly in the acceptance report
// (docs/gates/logs/rebrand-acceptance-4h2.log); the seam-to-screen path
// proven here is what makes that rewrite sufficient.

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsProvider } from "../SettingsProvider";
import { AboutArea } from "./updates-about";

// The fixture values from tests/rebrand/nova.toml, injected at the seam.
// Literals only: vi.mock factories are hoisted above every import.
vi.mock("../../../build-info", () => ({
  UI_PACKAGE_NAME: "Nova Player",
  UI_PACKAGE_VERSION: "0.2.0-nova",
}));

describe("AboutArea — rebrand acceptance (Nova fixture injected at the build-info seam)", () => {
  it("renders the Nova product name and version, never Lumen", () => {
    render(
      <SettingsProvider>
        <AboutArea />
      </SettingsProvider>,
    );
    expect(screen.getByTestId("about-package-name").textContent).toBe("Nova Player");
    expect(screen.getByTestId("about-version").textContent).toBe("0.2.0-nova");
    expect(screen.getByTestId("about-panel").getAttribute("data-about-package")).toBe(
      "Nova Player",
    );
    // Zero source identity on the surface (doc 07 §8.3: no Lumen outside
    // the allowlist).
    expect(screen.getByTestId("about-panel").textContent).not.toContain("Lumen");
  });
});
