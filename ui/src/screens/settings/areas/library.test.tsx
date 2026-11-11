// w26.b08 — Library area tests (the b02 sibling contract's library half):
// the routed area, the playlist-services fold mounted unchanged (the file
// is owned by another lane — never edited here), the schema truth (no
// library [[setting]] rows are seeded, so there is no GENERATED_SCREENS.
// library to embed), and the honest-surface rule for the §C9 frame rows.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { LibraryArea } from "./library";
import { GENERATED_SCREENS } from "../generated/Screens";
import { SETTINGS_ENTRIES } from "../generated/schema";

const renderRouted = (area: string) =>
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[`/settings/${area}`]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>
    </ClientProvider>,
  );

const mountArea = () =>
  render(
    <SettingsProvider>
      <LibraryArea />
    </SettingsProvider>,
  );

describe("LibraryArea — routing + the b02 sibling contract", () => {
  it("is routed at /settings/library under the area registry", () => {
    renderRouted("library");
    expect(document.querySelector('[data-settings-area="library"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: "Library" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("mounts the playlist-services fold content unchanged (w26 merge: playlist-services → library)", () => {
    const { container } = mountArea();
    const fold = container.querySelector<HTMLElement>('[data-testid="playlist-services-area"]');
    expect(fold).not.toBeNull();
    expect(container.querySelector('[data-testid="library-area"]')).toContainElement(fold);
    // The fold's own honest empty state renders (PlaylistServicesArea's
    // testid, not a duplicate of it).
    expect(container.querySelectorAll('[data-testid="settings-area-empty"]')).toHaveLength(1);
  });

  it("schema truth: no library [[setting]] rows are seeded → no GENERATED_SCREENS.library", () => {
    // The mission brief's "GENERATED_SCREENS.library rows" premise, pinned
    // as data: the generator emits screens only for areas with ui=true
    // rows, and the b02 seed added none for library (the sibling contract
    // calls this area "pure hand"). When the schema owner seeds library
    // rows, this assertion flips and the area must embed the generated
    // screen (see docs/waves/w26/logs/w26.b08.md).
    expect(SETTINGS_ENTRIES.filter((entry) => entry.screen === "library")).toEqual([]);
    expect("library" in GENERATED_SCREENS).toBe(false);
  });
});

describe("LibraryArea — honest surface (config-never-invents)", () => {
  it("renders NO dead §C9 Opening Files / Metadata & Artwork controls (no schema keys, no data source)", () => {
    mountArea();
    const frameLabels = [
      "Only allow one VLC window",
      "Apply to files opened from the file manager",
      "Add to the playlist instead of replacing it",
      "When opening a folder",
      "Show hidden files",
      "Fetch titles and artwork",
      "Use the media library",
    ];
    for (const label of frameLabels) {
      expect(
        screen.queryByText(label),
        `frame row "${label}" has no schema key → renders nothing`,
      ).toBeNull();
    }
    // No media-library path input + Browse combo either: the §C9 frame
    // carries none and no data source exists.
    expect(screen.queryByRole("button", { name: "Browse" })).toBeNull();
  });

  it("renders zero settings rows and zero bound controls (the fold is the whole surface)", () => {
    const { container } = mountArea();
    expect(container.querySelectorAll(".settings-field")).toHaveLength(0);
    expect(
      container.querySelectorAll("[data-setting], [data-setting-capability]"),
    ).toHaveLength(0);
  });
});
