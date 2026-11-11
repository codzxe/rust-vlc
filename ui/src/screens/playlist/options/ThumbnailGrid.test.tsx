// w26.b10 — ThumbnailGrid tests (thumbnail.svg): frame card anatomy, the
// quiet centered h:mm:ss footer (00:02:10 zero-padded per frame), honest
// unknowns ("—"), single-click play dispatch, empty honesty.

import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ThumbnailGrid } from "./ThumbnailGrid";
import { mountOptions } from "./optionsTestSupport";

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
