// /keyboard-shortcuts tests (Solstice v2 craft pass) — pins the audit fixes:
//   * calm backdrop while mounted, cleaned up on unmount (brief §1);
//   * FRIENDLY keycap labels: no raw "BrowserRefresh"/"VolumeMute"/"MediaNext"
//     spill anywhere on the screen; arrows render as glyphs (brief §2.9);
//   * NUMERIC bookmark ordering: Play/Set Bookmark 1, 2, 3 … 10 — never the
//     lexical 1, 10, 2 (brief §2.10);
//   * search filters across ALL groups by localized label, raw combo AND
//     the friendly cap text ("mute" finds VolumeMute).

import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { ShortcutsScreen } from "./ShortcutsScreen";

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
  afterEach(() => {
    delete document.documentElement.dataset.backdrop;
  });

  it("veils the backdrop to calm while mounted and cleans up on unmount (v2 §1)", () => {
    const { unmount } = renderScreen();
    expect(document.documentElement.dataset.backdrop).toBe("calm");
    unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("renders the four reference groups as anchored cards", () => {
    renderScreen();
    for (const title of ["Playback", "Navigation", "Volume", "Player variants"]) {
      const section = groupOf(title);
      expect(section).not.toBeNull();
      expect(section.querySelectorAll(".shortcuts__row").length).toBeGreaterThan(0);
    }
  });

  it("renders friendly keycap labels — no raw media-key spill (v2 §2.9)", () => {
    renderScreen();
    const caps = [...document.querySelectorAll(".shortcuts__keycap")].map((cap) => cap.textContent);
    expect(caps.length).toBeGreaterThan(50); // the full reference, not a stub
    for (const raw of ["BrowserRefresh", "BrowserStop", "BrowserNext", "VolumeMute", "MediaNext", "MediaPlayPause", "MediaPrev", "MediaStop", "VolumeDown", "VolumeUp", "WheelDown", "WheelUp"]) {
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

  it("sorts numbered bookmark series numerically: 1, 2, 3 … 10 (v2 §2.10)", () => {
    renderScreen();
    const navigation = groupOf("Navigation");
    const labels = rowLabels(navigation).filter((label) => label !== null && label.startsWith("Play Bookmark"));
    expect(labels.length).toBe(10);
    const numbers = labels.map((label) => Number(/[0-9]+$/.exec(label ?? "")![0]));
    expect(numbers).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    const setLabels = rowLabels(navigation).filter((label) => label !== null && label.startsWith("Set Bookmark"));
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
});
