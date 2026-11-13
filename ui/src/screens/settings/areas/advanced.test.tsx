// w26.k13 (j13 P2-5): the Advanced AREA component is DELETED — the b02
// registry retired it (advanced=true entries render behind each home
// screen's in-flow Advanced Options disclosure; the `advanced` slug
// redirects to `performance`, areas/index.tsx SETTINGS_AREA_REDIRECTS).
// The component-mount tests died with it; what stays is the ONE live
// routing behavior the retired slug still owns: the redirect.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SETTINGS_AREA_REDIRECTS } from "./index";

describe("retired `advanced` slug — redirect to its nearest home (w26.b02 fold)", () => {
  it("is redirected from /settings/advanced to performance via the registry (w26.b02 fold)", async () => {
    render(
      <ClientProvider transport={new MockTransport()}>
        <MemoryRouter initialEntries={["/settings/advanced"]}>
          <Routes>
            <Route path="/settings/:area" element={<SettingsScreen />} />
          </Routes>
        </MemoryRouter>
      </ClientProvider>,
    );
    // The Advanced AREA dissolved into the per-area in-flow disclosure
    // (w26.b02); the slug redirects to performance — its nearest home —
    // and the redirected screen's top-bar title announces it.
    expect(await screen.findByText("Performance")).toBeInTheDocument();
  });

  it("the registry maps the retired slug (and only via redirects — no area record)", () => {
    expect(SETTINGS_AREA_REDIRECTS.advanced).toBe("performance");
  });
});
