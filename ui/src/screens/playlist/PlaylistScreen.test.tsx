// w26.b09 — /playlist route body tests: the playlists LIBRARY screen
// (playlist.svg) + the create-playlist dialog (new playlist.svg).
//
// w26.f06 — extended for the shipped surfaces (e09 F1/F2/F3, e16 F3):
//  · the page grid anchors are pinned again (playlist.png: page, title,
//    near-black panel) — the column assertions dropped in the reskin.
//  · opening a library row (click / row-menu Open) mounts the BROWSE level:
//    the real toolbar (Sorting options + View options triggers), the view
//    switch (List View → media rows, Grid View → thumbnail cards) and the
//    sort re-order — all over the local saved-items scope.
//  · back returns to the library with the row still selected.
//  · the Add-subtitles action (toolbar + row menu) opens AddSubtitlesDialog.
//  · the browse empty state (a playlist with no recorded files).
//
// Contracts kept from b09: one heading + natural-sorted seed rows, the
// kebab/right-click row menu (Play disabled-honest, Remove deletes), the
// create dialog gates (Cancel/✕/Save), the honest library empty state.
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

const playlist = (id: string, title: string, itemNames: readonly string[] = []): SavedPlaylist => ({
  id,
  title,
  createdAt: 1,
  itemNames,
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
    .getAllByRole("row")
    .map((row) => row.querySelector(".lumen-lrow__title, .lumen-plo-mrow__title")?.textContent ?? "");

/** Opens the browse view for `title` by clicking its library row. */
const openBrowse = (title: string) => {
  fireEvent.click(screen.getByText(title));
};

describe("PlaylistScreen — library panel (playlist.svg)", () => {
  it("renders one page heading and the seeded frame playlists as rows", () => {
    render(<PlaylistScreen />);
    expect(screen.getAllByRole("heading", { name: "Playlist" }).length).toBe(1);
    expect(optionTitles()).toEqual(["Anime to Watch", "Cool Playlist", "Cool Songs"]);
    // count line — "3 playlists"
    expect(screen.getByText("3 playlists")).toBeTruthy();
  });

  // e16 F3 — the page-grid anchors dropped in the reskin, re-anchored on the
  // shipped surface (playlist.png: anchored column, display title, panel).
  it("pins the page grid anchors (playlist.png column structure)", () => {
    const { container } = render(<PlaylistScreen />);
    const page = container.querySelector("main.lumen-plpage");
    expect(page).not.toBeNull();
    expect(page!.getAttribute("data-screen")).toBe("playlist");
    const title = container.querySelector(".lumen-plpage__title");
    expect(title?.textContent).toBe("Playlist");
    const panel = container.querySelector(".lumen-plpage__panel");
    expect(panel).not.toBeNull();
    // the panel owns the rows grid (playlist.svg list surface; grid semantics since the w26 a11y fix)
    expect(panel!.querySelector(".lumen-plpage__list")).not.toBeNull();
  });

  it("natural-sorts numbered playlist titles (2 before 10, never lexical)", () => {
    seam.loaded = {
      playlists: [playlist("a", "Playlist 10"), playlist("b", "Playlist 2"), playlist("c", "Cool Playlist")],
    };
    resetPlaylistsForTests(seam);
    render(<PlaylistScreen />);
    expect(optionTitles()).toEqual(["Cool Playlist", "Playlist 2", "Playlist 10"]);
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

describe("PlaylistScreen — opened-playlist browse (w26.f06: thumbnail/media + option menus)", () => {
  it("opening a row mounts the browse level: title, item count, real sort/view triggers, media rows", () => {
    const { container } = render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    // the opened playlist is the page heading; the count reads items
    expect(screen.getAllByRole("heading", { name: "Anime to Watch" }).length).toBe(1);
    expect(screen.getByText("4 items")).toBeTruthy();
    expect(container.querySelector(".lumen-plpage")!.getAttribute("data-browse")).toBe("true");
    // the toolbar row: real triggers (F1) — sorting + view buttons
    expect(screen.getByRole("button", { name: "Sorting options" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "View options" })).toBeTruthy();
    // frame default (View Options.svg): List View → media rows, not cards
    expect(screen.getAllByRole("row").length).toBe(4);
    expect(screen.getByText("The Wand_S01E01.mp4")).toBeTruthy();
    expect(container.querySelector(".lumen-plo-card")).toBeNull();
  });

  it("View options → Grid View switches the body to the thumbnail grid (thumbnail.svg)", async () => {
    const { container } = render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    fireEvent.click(screen.getByRole("button", { name: "View options" }));
    const menu = screen.getByRole("menu", { name: "View options" });
    fireEvent.click(within(menu).getByText("Grid View"));
    await waitFor(() => {
      expect(container.querySelectorAll(".lumen-plo-card").length).toBe(4);
    });
    expect(screen.queryAllByRole("row")).toEqual([]);
    // saved items are names only — durations render the honest unknown
    expect(screen.getAllByText("—").length).toBe(4);
  });

  it("Sorting options → Title Z-A re-orders the browse rows", async () => {
    render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    fireEvent.click(screen.getByRole("button", { name: "Sorting options" }));
    const menu = screen.getByRole("menu", { name: "Sorting options" });
    // frame: 16 rows, row 1 checked (Unwatched First)
    expect(within(menu).getAllByRole("menuitemradio").length).toBe(16);
    fireEvent.click(within(menu).getByText("Title Z-A"));
    await waitFor(() => {
      expect(optionTitles()).toEqual([
        "The Wand_S02E02.mp4",
        "The Wand_S01E10.mp4",
        "The Wand_S01E02.mp4",
        "The Wand_S01E01.mp4",
      ]);
    });
  });

  it("back returns to the library with the opened row still selected", () => {
    render(<PlaylistScreen />);
    openBrowse("Cool Songs");
    expect(screen.getByText("3 items")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "All playlists" }));
    // library level again: rows + count, and the opened row kept its selection
    expect(screen.getByText("3 playlists")).toBeTruthy();
    const row = screen.getByText("Cool Songs").closest(".lumen-lrow")!;
    expect(row.getAttribute("aria-current")).toBe("true");
    expect(row.classList.contains("lumen-lrow--selected")).toBe(true);
  });

  it("a playlist with no recorded files renders the honest browse empty state", () => {
    render(<PlaylistScreen />);
    openBrowse("Cool Playlist");
    expect(screen.getByText("Nothing in this playlist yet")).toBeTruthy();
    expect(screen.getByText("Add files while creating a playlist to fill it.")).toBeTruthy();
    expect(screen.queryByRole("row")).toBeNull();
    expect(containerHasCards()).toBe(false);
  });

  it("saved media render as honest presentation — no play seam, real ••• menu (w26.gfix-E)", () => {
    const { container } = render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    // list view: rows carry the inert class (no play seam) and the ••• opens
    // the REAL file context menu (file context menu.svg — not a dead button)
    const inertRows = container.querySelectorAll(".lumen-plo-mrow--inert");
    expect(inertRows.length).toBe(4);
    expect(container.querySelector(".lumen-plo-mrow__menu")).not.toBeNull();
    // grid view: cards are presentation divs, never dead play buttons
    fireEvent.click(screen.getByRole("button", { name: "View options" }));
    fireEvent.click(within(screen.getByRole("menu", { name: "View options" })).getByText("Grid View"));
    expect(container.querySelectorAll("button.lumen-plo-card").length).toBe(0);
    expect(container.querySelectorAll("div.lumen-plo-card").length).toBe(4);
  });

  it("the media ••• opens the file context menu (file context menu.svg, 12 rows)", async () => {
    render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    fireEvent.click(screen.getAllByRole("button", { name: "More actions" })[0]!);
    const menu = await screen.findByRole("menu", { name: "File actions" });
    // Mark as Watched is a menuitemcheckbox — all 12 frame rows together,
    // in DOM order (frame row order).
    const items = [...menu.querySelectorAll<HTMLElement>('[role^="menuitem"]')];
    // 12 frame rows, frame order + frame copy.
    expect(items.map((item) => item.textContent)).toEqual([
      "Play",
      "Mark as Watched",
      "Favorite",
      "Stream",
      "Convert",
      "Add to Queue",
      "Add to Playlist",
      "Show in File Explorer",
      "Go to Source",
      "Add Subtitle Tracks",
      "Add Audio Tracks",
      "Properties & Metadata",
    ]);
    // Honest wiring: no-seam rows are disabled (Play has no catalog command).
    expect(within(menu).getByRole("menuitem", { name: "Play" }).getAttribute("aria-disabled")).toBe("true");
  });

  it("Mark as Watched toggles the row's watched state (media.svg watched state)", async () => {
    const { container } = render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    expect(container.querySelectorAll(".lumen-plo-mrow--watched").length).toBe(0);
    fireEvent.click(screen.getAllByRole("button", { name: "More actions" })[0]!);
    const menu = await screen.findByRole("menu", { name: "File actions" });
    const item = within(menu).getByRole("menuitemcheckbox", { name: "Mark as Watched" });
    expect(item.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(item);
    await waitFor(() => {
      expect(container.querySelectorAll(".lumen-plo-mrow--watched").length).toBeGreaterThan(0);
    });
  });

  it("Add to Playlist appends the file to another saved playlist (local store)", async () => {
    render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    fireEvent.click(screen.getAllByRole("button", { name: "More actions" })[0]!);
    const menu = await screen.findByRole("menu", { name: "File actions" });
    // Other playlists exist (Cool Playlist / Cool Songs) → submenu targets.
    const add = within(menu).getByRole("menuitem", { name: "Add to Playlist" });
    expect(add.getAttribute("aria-disabled")).toBeNull();
    fireEvent.click(add);
    const target = await within(document.body).findByRole("menuitem", { name: "Cool Songs" });
    fireEvent.click(target);
    // The store commit is observable: Cool Songs now carries the episode.
    fireEvent.click(screen.getByRole("button", { name: "All playlists" }));
    openBrowse("Cool Songs");
    expect(screen.getByText("The Wand_S01E01.mp4")).toBeTruthy();
  });

  it("Add Subtitle Tracks opens the dialog scoped to the one file (Folder-2)", async () => {
    render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    fireEvent.click(screen.getAllByRole("button", { name: "More actions" })[0]!);
    const menu = await screen.findByRole("menu", { name: "File actions" });
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Add Subtitle Tracks" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Add Subtitles")).toBeTruthy();
    // Folder-2's destination: "To This File"
    expect(within(dialog).getByRole("button", { name: "Apply subtitles to" }).textContent).toContain(
      "To This File",
    );
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("the row context menu's Open entry opens the browse view (same real seam as the click)", async () => {
    render(<PlaylistScreen />);
    fireEvent.click(screen.getAllByRole("button", { name: "Playlist actions" })[1]!); // Cool Playlist
    const menu = await screen.findByRole("menu");
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Open" }));
    expect(screen.getAllByRole("heading", { name: "Cool Playlist" }).length).toBe(1);
  });
});

describe("PlaylistScreen — Add-subtitles action (w26.f06, e09 F2)", () => {
  it("the playlist page toolbar opens the AddSubtitlesDialog scoped to the opened playlist", async () => {
    render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    fireEvent.click(screen.getByRole("button", { name: "Add subtitles" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Add Subtitles")).toBeTruthy();
    // no folder-scan seam: candidates render the honest empty state
    expect(within(dialog).getByText("No subtitle files found")).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("the library row menu's Add subtitles entry opens the same dialog", async () => {
    render(<PlaylistScreen />);
    fireEvent.click(screen.getAllByRole("button", { name: "Playlist actions" })[0]!);
    const menu = await screen.findByRole("menu");
    fireEvent.click(within(menu).getByRole("menuitem", { name: "Add subtitles" }));
    expect(await screen.findByRole("dialog")).toBeTruthy();
  });
});

/** Cards-present probe for the empty-state test (no container plumbing). */
function containerHasCards(): boolean {
  return document.querySelectorAll(".lumen-plo-card").length > 0;
}

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

  // w26.i10 (h16 dialog nit): initial focus belongs on the dialog's primary
  // control — the title input — not the shared trap's first-focusable (✕).
  // The refocus rides a macrotask (dev StrictMode re-runs the child trap
  // effect after the parent's), so the assertion waits for it.
  it("opening the dialog focuses the title input", async () => {
    render(<PlaylistScreen />);
    await openDialog();
    await waitFor(() =>
      expect(document.activeElement).toBe(screen.getByPlaceholderText("e.g., Favorite Videos")),
    );
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
    expect(screen.queryByRole("row")).toBeNull();
    // the CTA teaches the create flow (header CTA + empty-state action share
    // the label; either one opens the dialog)
    fireEvent.click(screen.getAllByRole("button", { name: "New playlist" })[0]!);
    expect(screen.getByRole("dialog")).toBeTruthy();
  });
});

// w26.i10 (h16 a11y shame #2) — the role=grid lists carry the APG grid
// keyboard model: focus stays on the grid container, aria-activedescendant +
// data-active point at the active row, ArrowUp/Down/Home/End move it and
// Enter/Space run the active row's real seam (library: open the playlist —
// previously div+onClick rows were keyboard-unreachable; media: open the
// row's file context menu).
describe("PlaylistScreen — arrow-key grid navigation (w26.i10)", () => {
  const libraryGrid = (container: HTMLElement): HTMLElement =>
    container.querySelector<HTMLElement>(".lumen-plpage__list")!;

  it("library grid: arrows/Home/End move aria-activedescendant over the rows", () => {
    const { container } = render(<PlaylistScreen />);
    const grid = libraryGrid(container);
    expect(grid.getAttribute("role")).toBe("grid");
    expect(grid.getAttribute("aria-activedescendant")).toBeNull(); // nothing active yet
    fireEvent.keyDown(grid, { key: "ArrowDown" });
    expect(grid.getAttribute("aria-activedescendant")).toContain("-row-0");
    expect(container.querySelector('[data-active="true"] .lumen-lrow__title')?.textContent).toBe(
      "Anime to Watch",
    );
    fireEvent.keyDown(grid, { key: "ArrowDown" });
    expect(grid.getAttribute("aria-activedescendant")).toContain("-row-1");
    fireEvent.keyDown(grid, { key: "ArrowUp" });
    expect(grid.getAttribute("aria-activedescendant")).toContain("-row-0");
    fireEvent.keyDown(grid, { key: "End" });
    expect(grid.getAttribute("aria-activedescendant")).toContain("-row-2");
    expect(container.querySelector('[data-active="true"] .lumen-lrow__title')?.textContent).toBe(
      "Cool Songs",
    );
    fireEvent.keyDown(grid, { key: "Home" });
    expect(grid.getAttribute("aria-activedescendant")).toContain("-row-0");
    // moves clamp at the ends (3 seeded playlists)
    fireEvent.keyDown(grid, { key: "ArrowUp" });
    expect(grid.getAttribute("aria-activedescendant")).toContain("-row-0");
  });

  it("library grid: Enter and Space open the active playlist (the click seam, keyboard-reachable)", () => {
    const first = render(<PlaylistScreen />);
    const grid = libraryGrid(first.container);
    fireEvent.keyDown(grid, { key: "ArrowDown" });
    fireEvent.keyDown(grid, { key: "ArrowDown" }); // row 1 = Cool Playlist
    fireEvent.keyDown(grid, { key: "Enter" });
    expect(screen.getAllByRole("heading", { name: "Cool Playlist" }).length).toBe(1);
    fireEvent.click(screen.getByRole("button", { name: "All playlists" }));
    // the active row survives the round-trip (row 1 still pointed at);
    // Home + Space activates row 0 on the re-mounted library grid
    const gridAgain = libraryGrid(first.container);
    expect(gridAgain.getAttribute("aria-activedescendant")).toContain("-row-1");
    fireEvent.keyDown(gridAgain, { key: "Home" });
    fireEvent.keyDown(gridAgain, { key: " " });
    expect(screen.getAllByRole("heading", { name: "Anime to Watch" }).length).toBe(1);
  });

  it("library grid: Enter without an active row is a no-op; keys from a row's kebab keep their own behavior", () => {
    const { container } = render(<PlaylistScreen />);
    const grid = libraryGrid(container);
    fireEvent.keyDown(grid, { key: "Enter" }); // nothing active — stays on the library
    expect(screen.getByText("3 playlists")).toBeTruthy();
    // the kebab's Enter opens the ROW MENU, never moves the active row
    const kebab = screen.getAllByRole("button", { name: "Playlist actions" })[0]!;
    fireEvent.keyDown(kebab, { key: "ArrowDown" });
    expect(grid.getAttribute("aria-activedescendant")).toBeNull();
  });

  it("media grid: arrows move the active row; Enter opens its file context menu", async () => {
    const { container } = render(<PlaylistScreen />);
    openBrowse("Anime to Watch");
    const grid = container.querySelector<HTMLElement>(".lumen-plo-media")!;
    expect(grid.getAttribute("role")).toBe("grid");
    fireEvent.keyDown(grid, { key: "ArrowDown" });
    expect(grid.getAttribute("aria-activedescendant")).toContain("-row-0");
    expect(container.querySelector('[data-active="true"] .lumen-plo-mrow__title')?.textContent).toBe(
      "The Wand_S01E01.mp4",
    );
    fireEvent.keyDown(grid, { key: "Enter" });
    // the row's one real seam in the saved-playlist scope: the file menu
    const menu = await screen.findByRole("menu", { name: "File actions" });
    expect(within(menu).getAllByRole("menuitem", { name: "Play" }).length).toBe(1);
  });
});
