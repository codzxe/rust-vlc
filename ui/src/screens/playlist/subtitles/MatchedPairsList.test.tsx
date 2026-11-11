// w26.b11 — matched-pairs review list tests (Folder.svg / Folder-1.svg
// body): pair row anatomy (media glyph + name · leader · subtitle glyph +
// name), natural order passthrough and the honest empty state.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { MatchedPairsList } from "./MatchedPairsList";

afterEach(cleanup);

const PAIRS = [
  { mediaName: "The_Wand_S01E01.mp4", subtitleName: "The_Wand_S01E01.srt" },
  { mediaName: "The_Wand_S01E02.mp4", subtitleName: "The_Wand_S01E02.srt" },
];

describe("MatchedPairsList — Folder.svg review rows", () => {
  it("renders a labelled list of pair items in the given order", () => {
    render(<MatchedPairsList pairs={PAIRS} />);
    const list = screen.getByRole("list", { name: "Matched subtitle pairs" });
    expect(list).not.toBeNull();
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(items[0]!.textContent).toContain("The_Wand_S01E01.mp4");
    expect(items[0]!.textContent).toContain("The_Wand_S01E01.srt");
  });

  it("each row carries the frame anatomy: media glyph · leader · subtitle glyph", () => {
    const { container } = render(<MatchedPairsList pairs={PAIRS} />);
    const row = container.querySelector(".lumen-plsubs-pair");
    expect(row).not.toBeNull();
    expect(row!.querySelectorAll(".lumen-plsubs-pair__icon")).toHaveLength(2);
    expect(row!.querySelectorAll(".lumen-plsubs-pair__leader")).toHaveLength(1);
    expect(row!.querySelectorAll(".lumen-plsubs-pair__name")).toHaveLength(2);
  });

  it("renders the honest empty state when nothing matched", () => {
    render(<MatchedPairsList pairs={[]} />);
    expect(screen.getByText("No subtitle files found")).not.toBeNull();
    expect(screen.queryByRole("list")).toBeNull();
  });

  it("honours the accessible-name override", () => {
    render(<MatchedPairsList pairs={PAIRS} label="Pairs" />);
    expect(screen.getByRole("list", { name: "Pairs" })).not.toBeNull();
  });
});
