// /keyboard-shortcuts tests (W26 Figma rebuild, Keyboard Shortcuts.svg) —
// pins the behavior contracts that survive the reskin EXACTLY as before:
//   * FRIENDLY keycap labels: no raw "BrowserRefresh"/"VolumeMute"/"MediaNext"
//     spill anywhere on the screen; arrows render as glyphs — now as ONE
//     keycap per binding (the frame's 270×33 combo cell), not a chip run;
//   * NUMERIC bookmark ordering: Play/Set Bookmark 1, 2, 3 … 10 — never the
//     lexical 1, 10, 2;
//   * curated keymap order of non-numbered rows (runs are surgical);
//   * search filters across ALL groups by localized label, raw combo AND
//     the friendly cap text ("mute" finds VolumeMute), with the frame's
//     "All Fields" scope as the default.
// Plus the new window grammar: floating dialog (top bar ✕, search pill,
// field-scope dropdown, Action/Hotkey/Global Hotkey columns, Cancel +
// disabled Save footer, per-row edit pencil → Settings → Hotkeys).

import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { ShortcutsScreen, filterGroups, buildShortcutGroups } from "./ShortcutsScreen";

const renderScreen = () =>
  render(
    <MemoryRouter>
      <ShortcutsScreen />
    </MemoryRouter>,
  );

const groupOf = (title: string) => screen.getByRole("heading", { name: title }).closest("section")!;

const rowLabels = (section: HTMLElement): string[] =>
  [...section.querySelectorAll(".shortcuts__row")].map((row) =>
    row.querySelector(".shortcuts__action")!.textContent,
  );

describe("ShortcutsScreen (/keyboard-shortcuts)", () => {
  it("renders the four reference groups as sections of the shortcuts window", () => {
    renderScreen();
    for (const title of ["Playback", "Navigation", "Volume", "Player variants"]) {
      const section = groupOf(title);
      expect(section).not.toBeNull();
      expect(section.querySelectorAll(".shortcuts__row").length).toBeGreaterThan(0);
    }
  });

  it("renders the window grammar: search pill, scope filter, column headers, footer", () => {
    renderScreen();
    // Search pill (placeholder contract unchanged from the pre-rebuild UI).
    expect(screen.getByPlaceholderText("Filter by action or key…")).toBeInTheDocument();
    // Field-scope dropdown shows the frame's default value.
    expect(screen.getByRole("button", { name: "Search scope" })).toHaveTextContent(
      "All Fields",
    );
    // Column headers.
    for (const header of ["Action", "Hotkey", "Global Hotkey"]) {
      expect(screen.getByText(header)).toBeInTheDocument();
    }
    // Footer: Cancel (active) + Save (disabled — read-only reference).
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    // Top-bar ✕ close affordance.
    expect(screen.getByRole("button", { name: "Close shortcuts" })).toBeInTheDocument();
  });

  it("renders friendly keycap labels — no raw media-key spill", () => {
    renderScreen();
    const caps = [...document.querySelectorAll(".shortcuts__keycap")].map(
      (cap) => cap.textContent,
    );
    expect(caps.length).toBeGreaterThan(50); // the full reference, not a stub
    for (const raw of [
      "BrowserRefresh",
      "BrowserStop",
      "BrowserNext",
      "VolumeMute",
      "MediaNext",
      "MediaPlayPause",
      "MediaPrev",
      "MediaStop",
      "VolumeDown",
      "VolumeUp",
      "WheelDown",
      "WheelUp",
    ]) {
      expect(caps).not.toContain(raw);
    }
    expect(caps).toContain("Browser Refresh");
    expect(caps).toContain("Mute");
    expect(caps).toContain("Media Next");
    expect(caps).toContain("Media Play/Pause");
    // arrows render as glyphs
    expect(caps).toContain("←");
    expect(caps).toContain("→");
  });

  it("renders one keycap per binding with the full combo (Ctrl+O, not chip runs)", () => {
    renderScreen();
    const combos = [...document.querySelectorAll(".shortcuts__combo")].map(
      (combo) => combo.textContent,
    );
    expect(combos).toContain("Ctrl+O");
    expect(combos).toContain("Space");
    // global-hotkey cells render empty (no global bindings in the keymaps)
    const firstRow = document.querySelector(".shortcuts__row")!;
    expect(firstRow.querySelectorAll(".shortcuts__keycap").length).toBe(2);
  });

  it("links each hotkey's edit pencil to Settings → Hotkeys", () => {
    renderScreen();
    const edit = document.querySelector("a.shortcuts__edit")!;
    expect(edit).not.toBeNull();
    expect(edit.getAttribute("href")).toBe("/settings/hotkeys");
    expect(edit.getAttribute("aria-label")).toContain("Customize these in Settings");
  });

  it("sorts numbered bookmark series numerically: 1, 2, 3 … 10", () => {
    renderScreen();
    const navigation = groupOf("Navigation");
    const labels = rowLabels(navigation).filter(
      (label) => label !== null && label.startsWith("Play Bookmark"),
    );
    expect(labels.length).toBe(10);
    const numbers = labels.map((label) => Number(/[0-9]+$/.exec(label ?? "")![0]));
    expect(numbers).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    const setLabels = rowLabels(navigation).filter(
      (label) => label !== null && label.startsWith("Set Bookmark"),
    );
    expect(setLabels.map((label) => Number(/[0-9]+$/.exec(label ?? "")![0]))).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
  });

  it("keeps the curated order of non-numbered rows (runs are surgical)", () => {
    renderScreen();
    const volume = groupOf("Volume");
    const labels = rowLabels(volume);
    // the audio section keeps its keymap order (delays/devices/tracks first,
    // then the mute/volume trio) — only numbered runs are reordered
    expect(labels.slice(0, 4)).toEqual([
      "Audio Delay Down",
      "Audio Delay Up",
      "Audio Device Cycle",
      "Audio Track Cycle",
    ]);
    const muteIndex = labels.indexOf("Mute");
    expect(labels.slice(muteIndex, muteIndex + 3)).toEqual(["Mute", "Volume Down", "Volume Up"]);
  });

  it("search filters across all groups (label, raw combo and friendly cap text)", () => {
    renderScreen();
    const search = screen.getByPlaceholderText("Filter by action or key…");
    fireEvent.change(search, { target: { value: "bookmark" } });
    expect(screen.queryByRole("heading", { name: "Playback" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Volume" })).toBeNull();
    const navigation = groupOf("Navigation");
    expect(within(navigation).getAllByText(/Bookmark/u).length).toBeGreaterThan(0);

    // friendly text search: "mute" finds the row whose raw key is VolumeMute
    fireEvent.change(search, { target: { value: "mute" } });
    expect(screen.getByRole("heading", { name: "Volume" })).toBeInTheDocument();
    expect(within(groupOf("Volume")).getByText("Mute")).toBeInTheDocument();

    // empty result is honest
    fireEvent.change(search, { target: { value: "zzzz-no-such-key" } });
    expect(screen.getByText("No shortcuts match your search.")).toBeInTheDocument();
  });

  it("scopes the search via the field filter (All Fields / Action / Hotkey)", () => {
    renderScreen();
    const search = screen.getByPlaceholderText("Filter by action or key…");

    // "Space" is a raw combo, not a label — All Fields finds it.
    fireEvent.change(search, { target: { value: "Space" } });
    expect(screen.getByRole("heading", { name: "Navigation" })).toBeInTheDocument();

    // Action scope: "Space" matches no label → no groups.
    fireEvent.change(search, { target: { value: "Space" } });
    const filter = screen.getByRole("button", { name: "Search scope" });
    fireEvent.click(filter);
    fireEvent.click(screen.getByRole("option", { name: "Action" }));
    expect(screen.queryByRole("heading", { name: "Navigation" })).toBeNull();
    expect(screen.getByText("No shortcuts match your search.")).toBeInTheDocument();

    // Hotkey scope: raw combo text matches again.
    fireEvent.click(screen.getByRole("button", { name: "Search scope" }));
    fireEvent.click(screen.getByRole("option", { name: "Hotkey" }));
    expect(screen.getByRole("heading", { name: "Navigation" })).toBeInTheDocument();
  });

  it("filterGroups scope behavior is pure and total (label-only / key-only)", () => {
    const groups = buildShortcutGroups();
    const labelOf = (key: { toString(): string }) => key.toString();
    // "J" is the audio-delay-down combo but no label contains a lone "j"
    // word… use a needle that discriminates: "Ctrl+O" (combo only).
    const comboOnly = filterGroups(groups, labelOf as never, "Ctrl+O", "hotkey");
    expect(comboOnly.length).toBeGreaterThan(0);
    const actionScoped = filterGroups(groups, labelOf as never, "Ctrl+O", "action");
    expect(actionScoped.length).toBe(0);
    // default scope = all (pre-rebuild contract).
    const all = filterGroups(groups, labelOf as never, "Ctrl+O");
    expect(all.length).toBeGreaterThan(0);
  });
});
