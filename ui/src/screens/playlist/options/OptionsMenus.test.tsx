// w26.b10 — SortingOptionsMenu + ViewOptionsMenu wrapper tests: frame data
// (16 rows / 2 rows, frame order, frame copy), defaults, and wiring into
// the provider state (selecting a sort re-orders sortedEntries).

import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SortingOptionsMenu } from "./SortingOptionsMenu";
import { ViewOptionsMenu } from "./ViewOptionsMenu";
import { PlaylistOptionsViews } from "./PlaylistOptionsToolbar";
import { usePlaylistOptions } from "./PlaylistOptionsProvider";
import { mountOptions, OPTION_ENTRIES } from "./optionsTestSupport";

const openSorting = (): HTMLElement => {
  fireEvent.click(screen.getByRole("button", { name: "Sorting options" }));
  return screen.getByRole("menu", { name: "Sorting options" });
};

const openView = (): HTMLElement => {
  fireEvent.click(screen.getByRole("button", { name: "View options" }));
  return screen.getByRole("menu", { name: "View options" });
};

/** Row LABELS (the `.lumen-menu__label` span) — the row's textContent also
 * carries the w26.k15 honest-degradation hint suffix on meta-dependent
 * rows, so frame-copy assertions target the label itself (j15 P3-11). */
const sortLabels = (): string[] =>
  screen.getAllByRole("menuitemradio").map((item) => item.querySelector(".lumen-menu__label")!.textContent);

describe("SortingOptionsMenu — frame data (Sorting Options.svg)", () => {
  it("lists all 16 frame options in row order with frame copy", () => {
    mountOptions(<SortingOptionsMenu />);
    openSorting();
    const labels = sortLabels();
    expect(labels).toHaveLength(16);
    expect(labels[0]).toBe("Unwatched First");
    expect(labels[1]).toBe("Watched First");
    expect(labels[2]).toBe("Newest First");
    expect(labels[3]).toBe("Oldest First");
    expect(labels[4]).toBe("Title A-Z");
    expect(labels[11]).toBe("Track Number Descending");
    expect(labels[14]).toBe("Disc Number Ascending");
    expect(labels[15]).toBe("Disc Number Descending");
  });

  it("frame default: Unwatched First is the checked row", () => {
    mountOptions(<SortingOptionsMenu />);
    openSorting();
    const checked = screen.getAllByRole("menuitemradio").filter((item) => item.getAttribute("aria-checked") === "true");
    expect(checked).toHaveLength(1);
    expect(checked[0]!.querySelector(".lumen-menu__label")!.textContent).toBe("Unwatched First");
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
      const rows = mounted.view.getAllByRole("row");
      expect(rows.map((row) => row.textContent)).toEqual([
        "The Wand_S01E01.mp4",
        "The Wand_S01E02.mp4",
        "The Wand_S01E10.mp4",
      ]);
    });
  });
});

describe("SortingOptionsMenu — metadata hint (w26.k15, j15 P3-11)", () => {
  it("without a metadata seam the meta-dependent rows carry the honest hint", () => {
    mountOptions(<SortingOptionsMenu />); // no resolveMeta, no watched marks
    openSorting();
    const items = screen.getAllByRole("menuitemradio");
    const hints = items.map((item) => item.querySelector(".lumen-plo__hint")?.textContent ?? null);
    // sortUsesMeta rows in frame order: 0/1 (watched/unwatched), 6–11
    // (artist/album/track), 14/15 (disc) — 10 of 16.
    expect(hints.filter((hint) => hint !== null)).toHaveLength(10);
    expect(hints[0]).toBe("Needs media metadata");
    expect(hints[15]).toBe("Needs media metadata");
    // Real sorts never hint: title (4/5), uri (12/13) — and newest/oldest
    // (2/3) ride the honest add-order proxy, so they stay unhinted too.
    expect(hints[2]).toBeNull();
    expect(hints[3]).toBeNull();
    expect(hints[4]).toBeNull();
    expect(hints[12]).toBeNull();
  });

  it("with the seam wired (the harness/frame mount shape) no row hints — the frame's plain rows", () => {
    mountOptions(<SortingOptionsMenu />, { resolveMeta: () => ({}) });
    openSorting();
    expect(
      screen.getAllByRole("menuitemradio").every((item) => item.querySelector(".lumen-plo__hint") === null),
    ).toBe(true);
  });

  it("a local Mark-as-Watched override also counts as wired (hints drop)", () => {
    const MarkAndSort = () => {
      const options = usePlaylistOptions();
      return (
        <>
          <button type="button" onClick={() => options.setWatched(OPTION_ENTRIES[0]!, true)}>
            mark
          </button>
          <SortingOptionsMenu />
        </>
      );
    };
    const mounted = mountOptions(<MarkAndSort />);
    act(() => mounted.view.getByRole("button", { name: "mark" }).click());
    openSorting();
    expect(
      screen.getAllByRole("menuitemradio").every((item) => item.querySelector(".lumen-plo__hint") === null),
    ).toBe(true);
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
    expect(checked[0]!.querySelector(".lumen-menu__label")!.textContent).toBe("List View");
  });

  it("selecting Grid View switches the shared body to the thumbnail grid", async () => {
    const mounted = mountOptions(
      <div>
        <ViewOptionsMenu />
        <PlaylistOptionsViews />
      </div>,
    );
    // Default: media list rows.
    expect(mounted.view.getAllByRole("row").length).toBe(3);
    openView();
    fireEvent.click(screen.getByText("Grid View"));
    await waitFor(() => expect(mounted.view.getAllByRole("list", { name: "Playlist thumbnails" }).length).toBe(1));
    expect(mounted.view.queryAllByRole("row")).toEqual([]);
  });
});
