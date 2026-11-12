// w26.b08 — Library area tests (the b02 sibling contract's library half):
// the routed area, the playlist-services fold mounted unchanged (the file
// is owned by another lane — never edited here), the schema truth (no
// library [[setting]] rows are seeded, so there is no GENERATED_SCREENS.
// library to embed), and the honest-surface rule for the §C9 frame rows.

import { fireEvent, render, screen } from "@testing-library/react";
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
    // w26.i15: the fold renders nothing today (its future schema rows' seam) —
    // no area-empty note (the library area carries the §C9 staged rows, so
    // "No options in this area yet." would be false under them) and zero
    // rows of its own.
    expect(fold!.querySelectorAll(".settings-field, [data-setting]")).toHaveLength(0);
    expect(container.querySelectorAll('[data-testid="settings-area-empty"]')).toHaveLength(0);
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

describe("LibraryArea — honest surface (the staged §C9 rows, w26.i15)", () => {
  it("renders the §C9 frame rows STAGED inert — visible, full-strength, never dead-but-alive", () => {
    const { container } = mountArea();
    // Opening Files (§C9 y83) — the frame's 5 rows, staged.
    for (const label of [
      "Only allow one VLC window",
      "Apply to files opened from the file manager",
      "Add to the playlist instead of replacing it",
      "When opening a folder",
      "Show hidden files",
    ]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    // Metadata & Artwork (§C9 y356) — the frame's 2 rows, staged.
    expect(screen.getByText("Fetch titles and artwork")).toBeInTheDocument();
    expect(screen.getByText("Use the media library")).toBeInTheDocument();
    // Frame-verbatim values on the staged controls.
    expect(screen.getByText("Include subfolders")).toBeInTheDocument();
    expect(screen.getByText("From the file only")).toBeInTheDocument();
    // Every staged row: the honest tooltip + the seam marker; the controls
    // are DISABLED (the frame's own §C0 disabled groups carry the dim marker).
    const rows = container.querySelectorAll("[data-staged-row]");
    expect(rows).toHaveLength(7);
    for (const row of rows) {
      expect(row.getAttribute("data-staged")).toBe("schema-seed");
      expect(row.getAttribute("title")).toContain("Staged");
    }
    expect(container.querySelectorAll('[data-staged-dimmed="true"]')).toHaveLength(2);
    expect(screen.getByRole("switch", { name: "Only allow one VLC window" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Use the media library" })).toBeDisabled();
    const folderOpen = screen.getByText("When opening a folder").closest(".settings-field");
    expect(folderOpen?.querySelector(".lumen-dropdown__trigger")).toBeDisabled();
  });

  it("carries the frame's ⓘ census (7/7 rows) operable — the reveal is the staged reason", () => {
    const { container } = mountArea();
    // SVG circle census (Settings - library.svg): 7 r7.5 #404040 circles —
    // every §C9 row, the two @0.3 dimmed rows included (they inherit the
    // group's opacity, ⓘ and all).
    const dots = container.querySelectorAll(".settings-field__info");
    expect(dots).toHaveLength(7);
    for (const dot of dots) {
      expect(dot.getAttribute("aria-label")).toContain("Staged");
      expect(dot.getAttribute("aria-expanded")).toBe("false");
    }
    // Operable (w26.i14 subtitles pattern): click reveals the hint line.
    fireEvent.click(dots[0]!);
    expect(dots[0]!.getAttribute("aria-expanded")).toBe("true");
    expect(container.querySelector(".settings-field__hinttext")?.textContent).toContain("Staged");
    expect(container.querySelector(".settings-field__hinttext")?.textContent).toContain(
      "library settings schema",
    );
    fireEvent.click(dots[0]!);
    expect(container.querySelector(".settings-field__hinttext")).toBeNull();
  });

  it("binds zero live settings rows (no schema keys — nothing pretends to be wired)", () => {
    const { container } = mountArea();
    expect(container.querySelectorAll("[data-setting], [data-setting-capability]")).toHaveLength(0);
    // The §C12-style media-library path input + Browse combo: the §C9 frame
    // carries none — nothing invented.
    expect(screen.queryByRole("button", { name: "Browse" })).toBeNull();
  });
});
