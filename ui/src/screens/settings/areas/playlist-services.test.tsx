// Card 4.E4 — Playlist & services discovery area tests: schema-missing
// options render NOTHING (config-never-invents), area is routed.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { PlaylistServicesArea } from "./playlist-services";

describe("PlaylistServicesArea — the seam (w26.i15: renders nothing today)", () => {
  it("renders zero rows and no area-root claim (no schema keys — never dead controls)", () => {
    const { container } = render(<PlaylistServicesArea />);
    // w26.i15: the fold no longer claims the library area root — exactly ONE
    // [data-settings-area="library"] renders per route (LibraryArea carries
    // it); no area-empty note under the staged §C9 rows.
    expect(container.querySelector('[data-settings-area]')).toBeNull();
    expect(container.querySelector('[data-testid="settings-area-empty"]')).toBeNull();
    expect(container.querySelectorAll(".settings-field")).toHaveLength(0);
    expect(container.querySelectorAll("[data-setting], [data-setting-capability]")).toHaveLength(0);
  });

  it("invents no service-discovery options the schema does not define", () => {
    render(<PlaylistServicesArea />);
    expect(screen.queryByText(/discovery/i)).toBeNull();
    expect(screen.queryByText(/UPnP|podcast| SAP /i)).toBeNull();
  });
});

describe("PlaylistServicesArea — routing", () => {
  it("is routed at /settings/library via the areas registry", () => {
    render(
      <ClientProvider transport={new MockTransport()}>
        <MemoryRouter initialEntries={["/settings/library"]}>
          <Routes>
            <Route path="/settings/:area" element={<SettingsScreen />} />
          </Routes>
        </MemoryRouter>
      </ClientProvider>,
    );
    expect(document.querySelector('[data-settings-area="library"]')).not.toBeNull();
  });
});
