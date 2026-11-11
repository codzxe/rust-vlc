// Natural-sort tests (w26.a18) — the digit-chunk compare and the bookmark
// row order it pins ("1, 2, 10 — not lexical", timecodes numeric).
import { describe, expect, it } from "vitest";
import type { Bookmark } from "../../../generated/commands";
import { compareBookmarks, naturalCompare } from "./naturalSort";

describe("naturalCompare — digit chunks compare numerically", () => {
  it("numbers inside text order 1, 2, 10 (not lexical)", () => {
    expect(naturalCompare("Chapter 2", "Chapter 10")).toBeLessThan(0);
    expect(naturalCompare("Chapter 10", "Chapter 2")).toBeGreaterThan(0);
    expect(naturalCompare("Take 1", "Take 2")).toBeLessThan(0);
    expect(naturalCompare("Take 2", "Take 10")).toBeLessThan(0);
  });

  it("plain strings stay byte-wise; equal strings are 0", () => {
    expect(naturalCompare("abc", "abd")).toBeLessThan(0);
    expect(naturalCompare("Intro", "Intro")).toBe(0);
  });

  it("prefix sorts first; leading zeros fold to their numeric value", () => {
    expect(naturalCompare("Track 2", "Track 2 remix")).toBeLessThan(0);
    expect(naturalCompare("Track 02", "Track 2")).toBe(0);
    expect(naturalCompare("Track 09", "Track 10")).toBeLessThan(0);
  });

  it("empty string sorts before anything", () => {
    expect(naturalCompare("", "A")).toBeLessThan(0);
    expect(naturalCompare("A", "")).toBeGreaterThan(0);
  });
});

describe("compareBookmarks — the row order pin", () => {
  const at = (time: number, label: string | null, id: number): Bookmark => ({
    id,
    time,
    label,
  });

  it("time ascending numerically — 2:00 after 0:10 even though '2' > '1' lexically", () => {
    const tenSeconds = at(10_000_000, null, 3);
    const twoMinutes = at(120_000_000, null, 1);
    expect(compareBookmarks(tenSeconds, twoMinutes)).toBeLessThan(0);
    expect(compareBookmarks(twoMinutes, tenSeconds)).toBeGreaterThan(0);
  });

  it("equal times: labels in natural order, unnamed ('') first", () => {
    expect(
      compareBookmarks(at(0, "Part 10", 1), at(0, "Part 2", 2)),
    ).toBeGreaterThan(0);
    expect(
      compareBookmarks(at(0, null, 1), at(0, "Named", 2)),
    ).toBeLessThan(0);
  });

  it("creation id is the stable final tie-break", () => {
    expect(
      compareBookmarks(at(5, "Same", 7), at(5, "Same", 2)),
    ).toBeGreaterThan(0);
  });
});
