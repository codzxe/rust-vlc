// w26.b10 — MediaList tests (media.svg): the four frame row states
// (watched / unwatched / hover CSS), the orange watched bar (full for
// watched, partial from the seam), the ••• hover affordance + its callback
// seam, single-click play, current-row marking, empty honesty.

import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MediaList } from "./MediaList";
import { PlaylistOptionsProvider } from "./PlaylistOptionsProvider";
import { OPTION_ENTRIES, OPTION_META, mountOptions } from "./optionsTestSupport";
import type { PlaylistEntry } from "../PlaylistProvider";

const rowOf = (title: string): HTMLElement => {
  const rows = screen.getAllByRole("option");
  return rows.find((row) => row.textContent?.includes(title))!;
};

describe("MediaList — frame anatomy (media.svg)", () => {
  it("renders one 51px row per entry (VirtualList pitch)", () => {
    const { view } = mountOptions(<MediaList />);
    expect(screen.getAllByRole("option")).toHaveLength(3);
    const rows = view.container.querySelectorAll(".lumen-plo-mrow");
    expect(rows).toHaveLength(3);
  });

  it("titles render in the rows (frame: The Wand_S01E01.mp4)", () => {
    mountOptions(<MediaList />);
    expect(screen.getByText("The Wand_S01E01.mp4")).toBeTruthy();
    expect(screen.getByText("The Wand_S01E02.mp4")).toBeTruthy();
    expect(screen.getByText("The Wand_S01E10.mp4")).toBeTruthy();
  });

  it("watched rows mark data-watched + dim class (frame watched state)", () => {
    const { view } = mountOptions(<MediaList />, { resolveMeta: (e) => OPTION_META[e.id] });
    const watched = view.container.querySelectorAll(".lumen-plo-mrow--watched");
    expect(watched).toHaveLength(2); // ids 2 + 3 in OPTION_META
    expect(watched[0]!.getAttribute("data-watched")).toBe("true");
  });

  it("the watched bar: full width for watched, partial from the seam, absent unwatched", () => {
    const { view } = mountOptions(<MediaList />, { resolveMeta: (e) => OPTION_META[e.id] });
    const bars = view.container.querySelectorAll<HTMLElement>(".lumen-plo-mrow__progress");
    expect(bars).toHaveLength(2);
    expect(bars[0]!.style.width).toBe("50%"); // id 2: watchedProgress 0.5
    expect(bars[1]!.style.width).toBe("100%"); // id 3: watched → completed bar
    const unwatched = rowOf("The Wand_S01E10.mp4");
    expect(unwatched.querySelector(".lumen-plo-mrow__progress")).toBeNull();
  });

  it("with no metadata seam every row renders honestly unwatched", () => {
    const { view } = mountOptions(<MediaList />);
    expect(view.container.querySelector(".lumen-plo-mrow--watched")).toBeNull();
    expect(view.container.querySelector(".lumen-plo-mrow__progress")).toBeNull();
  });
});

describe("MediaList — interaction", () => {
  it("one click on a row plays the entry (playlist.play-item dispatch)", () => {
    const { transport } = mountOptions(<MediaList />);
    fireEvent.click(screen.getByText("The Wand_S01E02.mp4"));
    const plays = transport.sent.filter((r) => r.cmd === "playlist.play-item");
    expect(plays).toHaveLength(1);
    expect(plays[0] && "args" in plays[0] ? plays[0].args : null).toEqual({ id: 2 });
  });

  it("the ••• button fires onEntryMenu and stops row play-through", () => {
    const onEntryMenu = vi.fn();
    const { transport } = mountOptions(<MediaList onEntryMenu={onEntryMenu} />);
    const row = rowOf("The Wand_S01E01.mp4");
    fireEvent.click(row.querySelector<HTMLButtonElement>(".lumen-plo-mrow__menu")!);
    expect(onEntryMenu).toHaveBeenCalledWith(OPTION_ENTRIES[2]);
    expect(transport.sent.filter((r) => r.cmd === "playlist.play-item")).toHaveLength(0);
  });

  it("without onEntryMenu no dead ••• button renders (honest affordance)", () => {
    const { view } = mountOptions(<MediaList />);
    expect(view.container.querySelector(".lumen-plo-mrow__menu")).toBeNull();
  });

  it("the current engine item gets aria-current + the current class", () => {
    const { transport, view } = mountOptions(<MediaList />);
    // Engine reports the current item (playlist.changed current-changed).
    act(() => {
      transport.emit({ gen: 1, seq: 1, kind: "playlist.changed", change: { kind: "current-changed", id: 2 } });
    });
    const current = view.container.querySelector(".lumen-plo-mrow--current");
    expect(current).not.toBeNull();
    expect(current!.textContent).toContain("The Wand_S01E02.mp4");
    expect(current!.getAttribute("aria-current")).toBe("true");
  });

  it("renders nothing when the queue is empty — the panel owns the empty state", () => {
    const { view } = mountOptions(<MediaList />, { entries: [] });
    expect(view.container.querySelector(".lumen-plo-media")).toBeNull();
  });
});

describe("MediaList — local scope (w26.f06: saved-playlist browse)", () => {
  const LOCAL: readonly PlaylistEntry[] = [
    { id: -1, title: "The Wand_S01E01.mp4", source: null, durationSeconds: null },
    { id: -2, title: "The Wand_S01E02.mp4", source: null, durationSeconds: null },
  ];

  it("renders rows from the local entries without any engine provider", () => {
    const { container } = render(
      <PlaylistOptionsProvider entries={LOCAL}>
        <MediaList />
      </PlaylistOptionsProvider>,
    );
    expect(container.querySelectorAll(".lumen-plo-mrow")).toHaveLength(2);
    expect(screen.getByText("The Wand_S01E01.mp4")).toBeTruthy();
  });

  it("no play seam → rows are honest presentation (inert, no •••, no current)", () => {
    const { container } = render(
      <PlaylistOptionsProvider entries={LOCAL}>
        <MediaList />
      </PlaylistOptionsProvider>,
    );
    const rows = [...container.querySelectorAll<HTMLElement>(".lumen-plo-mrow")];
    expect(rows.every((row) => row.classList.contains("lumen-plo-mrow--inert"))).toBe(true);
    expect(container.querySelector(".lumen-plo-mrow__menu")).toBeNull(); // no menu seam either
    expect(container.querySelector(".lumen-plo-mrow--current")).toBeNull(); // no engine current
  });
});
