// Card 4.C1 — keymap seam tests (doc 05 §8.1 semantics from keymap data).
import { describe, expect, it } from "vitest";
import {
  DEFAULT_KEYMAP,
  matchBindings,
  normalizeBindingKey,
  parseKeymap,
  shellActionFor,
} from "./keymap";

describe("parseKeymap", () => {
  it("parses the default seed excerpt (keymaps/default.toml lines 154-156)", () => {
    expect([...DEFAULT_KEYMAP.keys()]).toEqual([
      "fullscreen",
      "exit-fullscreen",
      "show-controls-fullscreen",
    ]);
    expect(DEFAULT_KEYMAP.get("fullscreen")).toMatchObject({
      key: "F",
      recipe: "fullscreen-toggle",
      mode: "player-focused",
    });
    expect(DEFAULT_KEYMAP.get("exit-fullscreen")).toMatchObject({
      key: "Esc",
      recipe: "exit-fullscreen-close-dialog",
    });
    expect(DEFAULT_KEYMAP.get("show-controls-fullscreen")).toMatchObject({
      key: "I",
      uiAction: "show-controls-fullscreen",
    });
  });

  it("rejects duplicate binding ids", () => {
    const src = `[a]
go = { key = "G" }
[b]
go = { key = "H" }
`;
    expect(() => parseKeymap(src)).toThrow(/duplicate binding id "go"/);
  });
});

describe("normalizeBindingKey", () => {
  it("normalizes plain keys to DOM shapes", () => {
    expect(normalizeBindingKey("F")).toEqual({ key: "f", modifiers: [] });
    expect(normalizeBindingKey("Esc")).toEqual({ key: "escape", modifiers: [] });
  });
  it("captures modifier combos without claiming them", () => {
    expect(normalizeBindingKey("Cmd+F")).toEqual({ key: "f", modifiers: ["meta"] });
    expect(normalizeBindingKey("Ctrl+Shift+P")).toEqual({
      key: "p",
      modifiers: ["control", "shift"],
    });
  });
  it("returns null on unknown modifiers — never guesses", () => {
    expect(normalizeBindingKey("Hyper+X")).toBeNull();
  });
});

describe("matchBindings (data-driven, not hardcoded)", () => {
  it("matches F / Escape / i to the seed bindings", () => {
    const f = matchBindings(DEFAULT_KEYMAP, {
      key: "f",
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    });
    expect(f.map((b) => b.id)).toEqual(["fullscreen"]);
    const esc = matchBindings(DEFAULT_KEYMAP, {
      key: "Escape",
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    });
    expect(esc.map((b) => b.id)).toEqual(["exit-fullscreen"]);
    const i = matchBindings(DEFAULT_KEYMAP, {
      key: "i",
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    });
    expect(i.map((b) => b.id)).toEqual(["show-controls-fullscreen"]);
  });

  it("does not fire plain-key bindings on modifier combos (4.C2 owns those)", () => {
    const hits = matchBindings(DEFAULT_KEYMAP, {
      key: "f",
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    });
    expect(hits).toEqual([]);
  });

  it("returns nothing for unbound keys", () => {
    const hits = matchBindings(DEFAULT_KEYMAP, {
      key: "z",
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    });
    expect(hits).toEqual([]);
  });
});

describe("shellActionFor (doc 05 §2.7 shell-target recipes)", () => {
  it("maps shell-owned recipes and ui-actions", () => {
    expect(shellActionFor(DEFAULT_KEYMAP.get("fullscreen")!)).toEqual({ kind: "toggle-fullscreen" });
    expect(shellActionFor(DEFAULT_KEYMAP.get("exit-fullscreen")!)).toEqual({
      kind: "exit-fullscreen",
    });
    expect(shellActionFor(DEFAULT_KEYMAP.get("show-controls-fullscreen")!)).toEqual({
      kind: "show-controls",
    });
  });
  it("forwards anything else to the 4.C2 seam", () => {
    expect(shellActionFor({ id: "x", key: "Q", command: "playback.stop" })).toEqual({
      kind: "forward",
      binding: { id: "x", key: "Q", command: "playback.stop" },
    });
  });
});
