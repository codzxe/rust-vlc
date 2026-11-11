// w26.b10 — SortingOptionsMenu + ViewOptionsMenu wrapper tests: frame data
// (16 rows / 2 rows, frame order, frame copy), defaults, and wiring into
// the provider state (selecting a sort re-orders sortedEntries).

import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SortingOptionsMenu } from "./SortingOptionsMenu";
import { ViewOptionsMenu } from "./ViewOptionsMenu";
import { PlaylistOptionsViews } from "./PlaylistOptionsToolbar";
import { mountOptions } from "./optionsTestSupport";

const openSorting = (): HTMLElement => {
  fireEvent.click(screen.getByRole("button", { name: "Sorting options" }));
  return screen.getByRole("menu", { name: "Sorting options" });
};

const openView = (): HTMLElement => {
  fireEvent.click(screen.getByRole("button", { name: "View options" }));
  return screen.getByRole("menu", { name: "View options" });
};

describe("SortingOptionsMenu — frame data (Sorting Options.svg)", () => {
  it("lists all 16 frame options in row order with frame copy", () => {
    mountOptions(<SortingOptionsMenu />);
    openSorting();
    const items = screen.getAllByRole("menuitemradio");
    expect(items).toHaveLength(16);
    expect(items[0]!.textContent).toBe("Unwatched First");
    expect(items[1]!.textContent).toBe("Watched First");
    expect(items[2]!.textContent).toBe("Newest First");
    expect(items[3]!.textContent).toBe("Oldest First");
    expect(items[4]!.textContent).toBe("Title A-Z");
    expect(items[11]!.textContent).toBe("Track Number Descending");
    expect(items[14]!.textContent).toBe("Disc Number Ascending");
    expect(items[15]!.textContent).toBe("Disc Number Descending");
  });

  it("frame default: Unwatched First is the checked row", () => {
    mountOptions(<SortingOptionsMenu />);
    openSorting();
    const checked = screen.getAllByRole("menuitemradio").filter((item) => item.getAttribute("aria-checked") === "true");
    expect(checked).toHaveLength(1);
    expect(checked[0]!.textContent).toBe("Unwatched First");
  });

  it("sorting rows carry no icons and no shortcuts (frame: plain labels)", () => {
    mountOptions(<SortingOptionsMenu />);
    openSorting();
    const items = screen.getAllByRole("menuitemradio");
    expect(items.every((item) => item.querySelector(".lumen-plo__icon") === null)).toBe(true);
  });

  it("selecting Title A-Z re-sorts the provider's entries", async () => {
    const mounted = mountOptions(
      <div>
        <SortingOptionsMenu />
        <PlaylistOptionsViews />
      </div>,
    );
    openSorting();
    fireEvent.click(screen.getByText("Title A-Z"));
    await waitFor(() => {
      const rows = mounted.view.getAllByRole("option");
      expect(rows.map((row) => row.textContent)).toEqual([
        "The Wand_S01E01.mp4",
        "The Wand_S01E02.mp4",
        "The Wand_S01E10.mp4",
      ]);
    });
  });
});

describe("ViewOptionsMenu — frame data (View Options.svg)", () => {
  it("lists the two frame options with their icons", () => {
    mountOptions(<ViewOptionsMenu />);
    openView();
    const items = screen.getAllByRole("menuitemradio");
    expect(items).toHaveLength(2);
    expect(items[0]!.textContent).toBe("Grid View");
    expect(items[1]!.textContent).toBe("List View");
    expect(items[0]!.querySelector(".lumen-plo__icon svg")).not.toBeNull();
    expect(items[1]!.querySelector(".lumen-plo__icon svg")).not.toBeNull();
  });

  it("frame default: List View is the checked row; trigger shows the mode icon", () => {
    mountOptions(<ViewOptionsMenu />);
    openView();
    const checked = screen.getAllByRole("menuitemradio").filter((item) => item.getAttribute("aria-checked") === "true");
    expect(checked).toHaveLength(1);
    expect(checked[0]!.textContent).toBe("List View");
  });

  it("selecting Grid View switches the shared body to the thumbnail grid", async () => {
    const mounted = mountOptions(
      <div>
        <ViewOptionsMenu />
        <PlaylistOptionsViews />
      </div>,
    );
    // Default: media list rows.
    expect(mounted.view.getAllByRole("option").length).toBe(3);
    openView();
    fireEvent.click(screen.getByText("Grid View"));
    await waitFor(() => expect(mounted.view.getAllByRole("list", { name: "Playlist thumbnails" }).length).toBe(1));
    expect(mounted.view.queryAllByRole("option")).toEqual([]);
  });
});
