// w26.b10 — provider tests: defaults (frame-selected rows), state changes,
// sortedEntries memoization, the metadata seam and the honest no-provider
// error.

import { act, render, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DEFAULT_VIEW_MODE, PlaylistOptionsProvider, usePlaylistOptions } from "./PlaylistOptionsProvider";
import { OPTION_ENTRIES, OPTION_META, OptionsProviders, mountOptions } from "./optionsTestSupport";
import { DEFAULT_SORT_KEY } from "./sorting";
import type { PlaylistEntry } from "../PlaylistProvider";

describe("PlaylistOptionsProvider — frame defaults", () => {
  it("default sort is the frame's checked row (Unwatched First)", () => {
    expect(DEFAULT_SORT_KEY).toBe("unwatched-first");
    const { result } = renderHook(() => usePlaylistOptions(), { wrapper: OptionsProviders });
    expect(result.current.sortKey).toBe("unwatched-first");
  });

  it("default view is the frame's checked row (List View)", () => {
    expect(DEFAULT_VIEW_MODE).toBe("list");
    const { result } = renderHook(() => usePlaylistOptions(), { wrapper: OptionsProviders });
    expect(result.current.viewMode).toBe("list");
  });

  it("initialSortKey/initialViewMode override the defaults", () => {
    const { result } = renderHook(() => usePlaylistOptions(), {
      wrapper: ({ children }) => (
        <OptionsProviders initialSortKey="title-asc" initialViewMode="grid">
          {children}
        </OptionsProviders>
      ),
    });
    expect(result.current.sortKey).toBe("title-asc");
    expect(result.current.viewMode).toBe("grid");
  });
});

describe("PlaylistOptionsProvider — state + sortedEntries", () => {
  it("setSortKey re-sorts sortedEntries (title-asc: S01E01 < S01E02 < S01E10)", async () => {
    const Consumer = () => {
      const options = usePlaylistOptions();
      return (
        <div>
          <button type="button" onClick={() => options.setSortKey("title-asc")}>
            apply
          </button>
          <output data-testid="ids">{options.sortedEntries.map((entry) => entry.id).join(",")}</output>
        </div>
      );
    };
    const mounted = mountOptions(<Consumer />);
    // Default (unwatched-first, no watched data) keeps queue order.
    await waitFor(() => expect(mounted.view.getByTestId("ids").textContent).toBe("1,2,3"));
    act(() => mounted.view.getByRole("button", { name: "apply" }).click());
    await waitFor(() => expect(mounted.view.getByTestId("ids").textContent).toBe("3,2,1"));
  });

  it("setViewMode switches grid/list", () => {
    const { result } = renderHook(() => usePlaylistOptions(), {
      wrapper: ({ children }) => (
        <OptionsProviders initialViewMode="list">{children}</OptionsProviders>
      ),
    });
    act(() => result.current.setViewMode("grid"));
    expect(result.current.viewMode).toBe("grid");
  });

  it("watched-first uses the metadata seam", async () => {
    const Consumer = () => {
      const options = usePlaylistOptions();
      return <output data-testid="ids">{options.sortedEntries.map((entry) => entry.id).join(",")}</output>;
    };
    const mounted = mountOptions(<Consumer />, {
      entries: OPTION_ENTRIES,
      resolveMeta: (e) => OPTION_META[e.id],
      initialSortKey: "watched-first",
    });
    // Watched ids (2, 3) first, unwatched (1) last.
    await waitFor(() => expect(mounted.view.getByTestId("ids").textContent).toBe("2,3,1"));
  });

  it("metaOf resolves per entry through the seam", () => {
    const { result } = renderHook(() => usePlaylistOptions(), {
      wrapper: ({ children }) => (
        <OptionsProviders resolveMeta={(e) => OPTION_META[e.id]}>{children}</OptionsProviders>
      ),
    });
    expect(result.current.metaOf(OPTION_ENTRIES[1]!)!.watched).toBe(true);
    expect(result.current.metaOf(OPTION_ENTRIES[0]!)!.watched).toBeUndefined();
  });
});

describe("PlaylistOptionsProvider — context guard", () => {
  it("usePlaylistOptions outside a provider throws the honest error", () => {
    // Silence the expected React error boundary noise in the test output.
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      expect(() => renderHook(() => usePlaylistOptions())).toThrow(/PlaylistOptionsProvider/);
    } finally {
      spy.mockRestore();
    }
  });
});

describe("PlaylistOptionsProvider — local entries scope (w26.f06)", () => {
  const LOCAL_ENTRIES: readonly PlaylistEntry[] = [
    { id: -1, title: "The Wand_S01E10.mp4", source: null, durationSeconds: null },
    { id: -2, title: "The Wand_S01E02.mp4", source: null, durationSeconds: null },
    { id: -3, title: "The Wand_S01E01.mp4", source: null, durationSeconds: null },
  ];

  it("entries prop is the data source — no engine PlaylistProvider required", () => {
    const { result } = renderHook(() => usePlaylistOptions(), {
      wrapper: ({ children }) => (
        <PlaylistOptionsProvider entries={LOCAL_ENTRIES}>{children}</PlaylistOptionsProvider>
      ),
    });
    // Default sort keeps local order (unwatched-first, no metadata).
    expect(result.current.sortedEntries.map((e) => e.title)).toEqual([
      "The Wand_S01E10.mp4",
      "The Wand_S01E02.mp4",
      "The Wand_S01E01.mp4",
    ]);
  });

  it("the sort keys order the local scope (title-asc natural: 02 before 10)", async () => {
    const Consumer = () => {
      const options = usePlaylistOptions();
      return <output data-testid="ids">{options.sortedEntries.map((e) => e.id).join(",")}</output>;
    };
    const view = render(
      <PlaylistOptionsProvider entries={LOCAL_ENTRIES} initialSortKey="title-asc">
        <Consumer />
      </PlaylistOptionsProvider>,
    );
    await waitFor(() => expect(view.getByTestId("ids").textContent).toBe("-3,-2,-1"));
  });
});
