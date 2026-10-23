import { describe, expect, it } from "vitest";
import { detectConflicts } from "./conflicts";

// Card 4.E2 conflict detection: same normalized combo + same mode ⇒ both
// rows warn; different modes never collide; unparseable spellings never guess.

const B = (id: string, key: string, mode = "player-focused") => ({ id, key, mode });

describe("detectConflicts", () => {
  it("marks every row of a duplicated (mode, combo) group", () => {
    const ids = detectConflicts([B("a", "Ctrl+O"), B("b", "Ctrl+O")], {});
    expect([...ids].sort()).toEqual(["a", "b"]);
  });

  it("normalizes spellings before comparing (Esc ≡ escape, case, spacing)", () => {
    const ids = detectConflicts([B("a", "Esc"), B("b", "esc"), B("c", "F")], {});
    expect([...ids].sort()).toEqual(["a", "b"]);
  });

  it("honors draft overrides, not just defaults", () => {
    const ids = detectConflicts([B("a", "S"), B("b", "X")], { b: "S" });
    expect([...ids].sort()).toEqual(["a", "b"]);
  });

  it("never conflicts across activation modes", () => {
    const ids = detectConflicts([B("a", "Ctrl+Q", "always"), B("b", "Ctrl+Q")], {});
    expect(ids.size).toBe(0);
  });

  it("never conflicts on unparseable spellings (no guessing)", () => {
    const ids = detectConflicts([B("a", "Hyper+X"), B("b", "Hyper+X")], {});
    expect(ids.size).toBe(0);
  });

  it("treats modifier order as irrelevant (Ctrl+Shift+P ≡ Shift+Ctrl+P)", () => {
    const ids = detectConflicts([B("a", "Ctrl+Shift+P"), B("b", "Shift+Ctrl+P")], {});
    expect([...ids].sort()).toEqual(["a", "b"]);
  });

  it("clears when the duplicate is resolved", () => {
    const ids = detectConflicts([B("a", "Ctrl+O"), B("b", "Ctrl+O")], { b: "Ctrl+K" });
    expect(ids.size).toBe(0);
  });
});
