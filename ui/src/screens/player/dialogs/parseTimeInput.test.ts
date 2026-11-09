// Go-to-Time input parser tests (issue #32) — the accepted grammar,
// pinned field by field; everything else is honestly null.
import { describe, expect, it } from "vitest";
import { parseTimeInput } from "./parseTimeInput";

describe("parseTimeInput — accepted forms", () => {
  it("bare seconds, optionally fractional", () => {
    expect(parseTimeInput("90")).toBe(90);
    expect(parseTimeInput("90.5")).toBe(90.5);
    expect(parseTimeInput("0")).toBe(0);
    expect(parseTimeInput("  65  ")).toBe(65);
  });

  it("mm:ss (minutes may exceed 59, VLC parity)", () => {
    expect(parseTimeInput("1:30")).toBe(90);
    expect(parseTimeInput("0:00")).toBe(0);
    expect(parseTimeInput("0:59")).toBe(59);
    expect(parseTimeInput("90:00")).toBe(5400);
  });

  it("hh:mm:ss", () => {
    expect(parseTimeInput("1:02:05")).toBe(3725);
    expect(parseTimeInput("0:00:00")).toBe(0);
    expect(parseTimeInput("2:00:00")).toBe(7200);
    expect(parseTimeInput("10:11:12")).toBe(36672);
  });
});

describe("parseTimeInput — honest refusals (null, never guessed)", () => {
  it("empty / whitespace-only input", () => {
    expect(parseTimeInput("")).toBeNull();
    expect(parseTimeInput("   ")).toBeNull();
  });

  it("negatives and signs — a goto target is a position, not a delta", () => {
    expect(parseTimeInput("-5")).toBeNull();
    expect(parseTimeInput("-1:30")).toBeNull();
    expect(parseTimeInput("+90")).toBeNull();
  });

  it("non-numeric / fractional low fields in colon forms", () => {
    expect(parseTimeInput("abc")).toBeNull();
    expect(parseTimeInput("1:3o")).toBeNull();
    expect(parseTimeInput("1:30.5")).toBeNull();
    expect(parseTimeInput("1.5:30")).toBeNull();
    expect(parseTimeInput("one:two")).toBeNull();
  });

  it("out-of-range low fields (0..59) and 4+ fields", () => {
    expect(parseTimeInput("1:60")).toBeNull();
    expect(parseTimeInput("1:02:60")).toBeNull();
    expect(parseTimeInput("1:02:03:04")).toBeNull();
  });
});
