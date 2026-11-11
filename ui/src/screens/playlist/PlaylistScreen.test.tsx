// w26.b09 — /playlist route body tests: the playlists LIBRARY screen
// (playlist.svg) + the create-playlist dialog (new playlist.svg).
//
// Contracts pinned here:
//  · the page renders ONE heading and the near-black panel of natural-sorted
//    ListRow options (seed fixture rows; "Playlist 2" before "Playlist 10").
//  · row selection (aria-current + the selected row class).
//  · the kebab `•••` opens the SAME context menu as right-click; Play is
//    disabled-honest (no catalog command), Remove deletes the saved row.
//  · the create dialog: Cancel/✕ close, Save gated on a non-empty title,
//    Save creates + persists the playlist (seam-captured).
//  · the honest empty state (no invented playlists) with the New-playlist
//    teaching CTA.
// The queue panel's own contracts live in PlaylistPanel.test.tsx.

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PlaylistScreen } from "./PlaylistScreen";
import {
  resetPlaylistsForTests,
  type PlaylistsPersistence,
  type PlaylistsState,
  type SavedPlaylist,
} from "./playlistsStore";

/** Controllable seam: preloads state, captures every commit. */
class CapturingSeam implements PlaylistsPersistence {
  loaded: { playlists?: unknown } | null = null;
  readonly saved: PlaylistsState[] = [];

  load() {
    return this.loaded;
  }

  save(state: PlaylistsState): void {
    this.saved.push(state);
  }
}

const playlist = (id: string, title: string): SavedPlaylist => ({
  id,
  title,
  createdAt: 1,
  itemNames: [],
});

let seam: CapturingSeam;

beforeEach(() => {
  seam = new CapturingSeam();
  resetPlaylistsForTests(seam);
});

afterEach(() => {
  resetPlaylistsForTests(); // back to the localStorage seam for other suites
});

const optionTitles = (): string[] =>
  screen
    .getAllByRole("option")
    .map((row) => row.querySelector(".lumen-lrow__title")?.textContent ?? "");

describe("PlaylistScreen — library panel (playlist.svg)", () => {
  it("renders one page heading and the seeded frame playlists as rows", () => {
    render(<PlaylistScreen />);
    expect(screen.getAllByRole("heading", { name: "Playlist" }).length).toBe(1);
    expect(optionTitles()).toEqual(["Anime to Watch", "Cool Playlist", "Cool Songs"]);
    // count line — "3 playlists"
    expect(screen.getByText("3 playlists")).toBeTruthy();
  });

  it("natural-sorts numbered playlist titles (2 before 10, never lexical)", () => {
    seam.loaded = {
      playlists: [playlist("a", "Playlist 10"), playlist("b", "Playlist 2"), playlist("c", "Cool Playlist")],
    };
    resetPlaylistsForTests(seam);
    render(<PlaylistScreen />);
    expect(optionTitles()).toEqual(["Cool Playlist", "Playlist 2", "Playlist 10"]);
  });

  it("selects a row on click (aria-current + the selected row class)", () => {
    render(<PlaylistScreen />);
    fireEvent.click(screen.getByText("Cool Playlist"));
    const row = screen.getByText("Cool Playlist").closest(".lumen-lrow")!;
    expect(row.getAttribute("aria-current")).toBe("true");
    expect(row.classList.contains("lumen-lrow--selected")).toBe(true);
    // exactly one selected row
    expect(document.querySelectorAll(".lumen-lrow--selected").length).toBe(1);
  });

  it("kebab opens the row context menu; Play is disabled-honest, Remove deletes", async () => {
    render(<PlaylistScreen />);
    fireEvent.click(screen.getAllByRole("button", { name: "Playlist actions" })[0]!);
    const menu = await screen.findByRole("menu");
    const play = within(menu).getByRole("menuitem", { name: "Play" });
    expect(play.getAttribute("aria-disabled")).toBe("true"); // no catalog command
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Remove" }));
    await waitFor(() => {
      expect(screen.queryByText("Anime to Watch")).toBeNull(); // first row removed
    });
    expect(optionTitles().length).toBe(2);
  });
});

describe("PlaylistScreen — create-playlist dialog (new playlist.svg)", () => {
  const openDialog = () => {
    fireEvent.click(screen.getByRole("button", { name: "New playlist" }));
    return screen.findByRole("dialog");
  };
  it("opens from the header CTA and closes on Cancel", async () => {
    render(<PlaylistScreen />);
    await openDialog();
    expect(screen.getByText("Playlist details")).toBeTruthy();
    expect(screen.getByText("Select files")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("closes on the ✕ affordance", async () => {
    render(<PlaylistScreen />);
    await openDialog();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("Save stays disabled until the title is non-empty", async () => {
    render(<PlaylistScreen />);
    await openDialog();
    const save = screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    fireEvent.change(screen.getByPlaceholderText("e.g., Favorite Videos"), {
      target: { value: "Road movies" },
    });
    expect(save.disabled).toBe(false);
  });

  it("Save creates the playlist, closes and persists through the seam", async () => {
    render(<PlaylistScreen />);
    await openDialog();
    fireEvent.change(screen.getByPlaceholderText("e.g., Favorite Videos"), {
      target: { value: "Road movies" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(screen.getByText("Road movies")).toBeTruthy());
    // persisted: the seam captured a commit containing the created playlist
    const last = seam.saved.at(-1);
    expect(last?.playlists.some((p) => p.title === "Road movies")).toBe(true);
  });

  it("the thumbnail picker is disabled-honest (no backend, titled hint)", async () => {
    render(<PlaylistScreen />);
    await openDialog();
    const thumb = screen.getByRole("button", { name: "Playlist thumbnail" }) as HTMLButtonElement;
    expect(thumb.disabled).toBe(true);
    expect(thumb.title).toContain("not exposed");
  });

  it("the drop zone is the file input's label (click-to-select seam)", async () => {
    render(<PlaylistScreen />);
    await openDialog();
    const input = screen.getByLabelText("Select files", { selector: "input" });
    expect(input.getAttribute("type")).toBe("file");
    expect(document.querySelector("label.lumen-plnew__drop")!.getAttribute("for")).toBe(
      input.id,
    );
  });
});

describe("PlaylistScreen — honest empty state", () => {
  it("a stored empty library renders the quiet empty state + teaching CTA", () => {
    seam.loaded = { playlists: [] };
    resetPlaylistsForTests(seam);
    render(<PlaylistScreen />);
    expect(screen.getByText("No playlists yet")).toBeTruthy();
    expect(screen.getByText("Playlists you create are saved on this device.")).toBeTruthy();
    expect(screen.queryByRole("option")).toBeNull();
    // the CTA teaches the create flow (header CTA + empty-state action share
    // the label; either one opens the dialog)
    fireEvent.click(screen.getAllByRole("button", { name: "New playlist" })[0]!);
    expect(screen.getByRole("dialog")).toBeTruthy();
  });
});
