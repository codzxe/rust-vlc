// Card 4.D1 — queue semantics unit tests (doc 03 §4/§810 + doc 05 §2.9 are
// the contract; the catalog is never second-guessed here).
import { describe, expect, it } from "vitest";
import {
  cycleRepeat,
  dragToMoveIndex,
  formatDuration,
  indexOfId,
  playNextTargetIndex,
  sourceLocation,
} from "./queue";

describe("cycleRepeat (VLC-like order)", () => {
  it("cycles off → all → one → off", () => {
    expect(cycleRepeat("off")).toBe("all");
    expect(cycleRepeat("all")).toBe("one");
    expect(cycleRepeat("one")).toBe("off");
  });
});

describe("playNextTargetIndex (play-next = insertion after current)", () => {
  it("inserts right after the current item", () => {
    expect(playNextTargetIndex(2, 5)).toBe(3);
  });
  it("appends when nothing is current (no 'next' exists)", () => {
    expect(playNextTargetIndex(-1, 5)).toBe(5);
  });
  it("clamps at the end when current is the last item", () => {
    expect(playNextTargetIndex(4, 5)).toBe(5);
  });
});

describe("dragToMoveIndex (drag → playlist.move to_index)", () => {
  it("dragging down shifts the target by one (post-removal coords)", () => {
    expect(dragToMoveIndex(4, 1, 5)).toBe(3);
  });
  it("dragging up keeps the target index", () => {
    expect(dragToMoveIndex(0, 3, 5)).toBe(0);
  });
  it("dropping onto the same slot is a no-op index", () => {
    expect(dragToMoveIndex(2, 2, 5)).toBe(2);
  });
  it("clamps out-of-range drop targets", () => {
    expect(dragToMoveIndex(9, 0, 5)).toBe(4);
    expect(dragToMoveIndex(-3, 2, 5)).toBe(0);
  });
});

describe("formatDuration / sourceLocation / indexOfId", () => {
  it("formats m:ss and h:mm:ss", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(65)).toBe("1:05");
    expect(formatDuration(3725)).toBe("1:02:05");
  });
  it("keeps unknown/invalid durations null (honest unknown)", () => {
    expect(formatDuration(null)).toBeNull();
    expect(formatDuration(Number.NaN)).toBeNull();
    expect(formatDuration(-1)).toBeNull();
  });
  it("renders file paths and stream urls as the location column", () => {
    expect(sourceLocation({ kind: "file", path: "/a/b.mkv" })).toBe("/a/b.mkv");
    expect(sourceLocation({ kind: "url", url: "https://x/y" })).toBe("https://x/y");
    expect(sourceLocation(null)).toBeNull();
  });
  it("finds stable ids, -1 when absent", () => {
    expect(indexOfId([7, 3, 9], 3)).toBe(1);
    expect(indexOfId([7, 3, 9], 4)).toBe(-1);
  });
});
