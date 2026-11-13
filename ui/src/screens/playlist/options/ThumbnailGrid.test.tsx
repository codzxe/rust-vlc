// w26.b10 — ThumbnailGrid tests (thumbnail.svg): frame card anatomy, the
// quiet centered h:mm:ss footer (00:02:10 zero-padded per frame), honest
// unknowns ("—"), single-click play dispatch, empty honesty.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ThumbnailGrid } from "./ThumbnailGrid";
import { PlaylistOptionsProvider } from "./PlaylistOptionsProvider";
import { mountOptions } from "./optionsTestSupport";
import type { PlaylistEntry } from "../PlaylistProvider";

describe("ThumbnailGrid — frame anatomy (thumbnail.svg)", () => {
  it("renders one card per entry in a labelled thumbnail grid", () => {
    const { view } = mountOptions(<ThumbnailGrid />);
    const grid = screen.getByRole("list", { name: "Playlist thumbnails" });
    expect(grid).toBeTruthy();
    expect(view.container.querySelectorAll(".lumen-plo-card")).toHaveLength(3);
  });

  it("cards carry the quiet centered h:mm:ss duration (00:02:10 frame form)", () => {
    mountOptions(<ThumbnailGrid />);
    expect(screen.getByText("00:02:10")).toBeTruthy(); // 130s
    expect(screen.getByText("00:42:59")).toBeTruthy(); // 2579s — frame keeps the 00 hours
    expect(screen.getByText("—")).toBeTruthy(); // null duration — honest unknown
  });

  it("titles resolve: title → source basename → unknown (title attribute)", () => {
    const { view } = mountOptions(<ThumbnailGrid />);
    const cards = [...view.container.querySelectorAll<HTMLButtonElement>(".lumen-plo-card")];
    expect(cards.every((card) => card.title.includes("The Wand_S01E"))).toBe(true);
  });

  it("w26.k15 (j15 P3-4): play cards are named by their TITLE, not the duration text", () => {
    mountOptions(<ThumbnailGrid />);
    // Name-from-content used to beat the `title` attr — every card announced
    // "00:02:10, button". aria-label wins over content; the duration stays
    // visible text.
    expect(screen.getByRole("button", { name: "The Wand_S01E10.mp4" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "The Wand_S01E01.mp4" })).toBeTruthy();
    const named = screen.getAllByRole("button").map((card) => card.getAttribute("aria-label"));
    expect(named.every((name) => !/^\d\d:\d\d:\d\d$/.test(name ?? ""))).toBe(true);
  });
});

describe("ThumbnailGrid — interaction", () => {
  it("one click plays the entry (playlist.play-item dispatch)", () => {
    const { transport } = mountOptions(<ThumbnailGrid />);
    fireEvent.click(screen.getByText("00:02:10"));
    const plays = transport.sent.filter((r) => r.cmd === "playlist.play-item");
    expect(plays).toHaveLength(1);
    expect(plays[0] && "args" in plays[0] ? plays[0].args : null).toEqual({ id: 1 });
  });

  it("renders nothing (no empty grid) when the queue is empty — the panel owns the empty state", () => {
    const { view } = mountOptions(<ThumbnailGrid />, { entries: [] });
    expect(view.container.querySelector(".lumen-plo-grid")).toBeNull();
  });
});

describe("ThumbnailGrid — local scope (w26.f06: saved-playlist browse)", () => {
  const LOCAL: readonly PlaylistEntry[] = [
    { id: -1, title: "The Wand_S01E01.mp4", source: null, durationSeconds: null },
    { id: -2, title: "The Wand_S01E02.mp4", source: null, durationSeconds: 130 },
  ];

  it("renders cards from the local entries without any engine provider", () => {
    const { container } = render(
      <PlaylistOptionsProvider entries={LOCAL}>
        <ThumbnailGrid />
      </PlaylistOptionsProvider>,
    );
    expect(container.querySelectorAll(".lumen-plo-card")).toHaveLength(2);
    expect(screen.getByText("00:02:10")).toBeTruthy(); // 130s local duration
  });

  it("no play seam → cards are honest presentation, never dead buttons", () => {
    const { container } = render(
      <PlaylistOptionsProvider entries={LOCAL}>
        <ThumbnailGrid />
      </PlaylistOptionsProvider>,
    );
    expect(container.querySelectorAll("button.lumen-plo-card")).toHaveLength(0);
    expect(container.querySelectorAll("div.lumen-plo-card--inert")).toHaveLength(2);
  });

  it("w26.k15 (j15 P3-4/P3-8): inert cards still carry an accessible name (the media title)", () => {
    render(
      <PlaylistOptionsProvider entries={LOCAL}>
        <ThumbnailGrid />
      </PlaylistOptionsProvider>,
    );
    // role="img" + aria-label — a bare div's aria-label would be ignored and
    // the card would announce its duration text (or nothing at all).
    expect(screen.getByRole("img", { name: "The Wand_S01E01.mp4" })).toBeTruthy();
    expect(screen.getByRole("img", { name: "The Wand_S01E02.mp4" })).toBeTruthy();
  });
});
