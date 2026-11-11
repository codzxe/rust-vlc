// Card 4.C1 — keymap seam tests (doc 05 §8.1 semantics from keymap data).
import { describe, expect, it } from "vitest";
import {
  DEFAULT_KEYMAP,
  DIALOG_KEYMAP,
  matchBindings,
  matchModifierBindings,
  normalizeBindingKey,
  parseKeymap,
  shellActionFor,
} from "./keymap";
import { PLAYER_SHELL_KEYMAP } from "./variants";

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

describe("DIALOG_KEYMAP seed (issue #32 — verbatim keymaps/default.toml rows)", () => {
  it("carries jump-to-time and bookmarks-dialog exactly as the keymap file spells them", () => {
    expect(DIALOG_KEYMAP.get("jump-to-time")).toMatchObject({
      key: "Ctrl+T",
      uiAction: "goto-time",
      mode: "player-focused",
      when: "has-media",
    });
    expect(DIALOG_KEYMAP.get("bookmarks-dialog")).toMatchObject({
      key: "Ctrl+B",
      uiAction: "bookmarks",
      mode: "player-focused",
    });
  });

  it("carries effects-and-filters exactly as the keymap file spells it (w26.d1, VLC Ctrl+E)", () => {
    // keymaps/default.toml [window] effects-and-filters: QtHotkeys 'Extended
    // settings Ctrl+E' (Adjustments and Effects) → the Tools dialog.
    expect(DIALOG_KEYMAP.get("effects-and-filters")).toMatchObject({
      key: "Ctrl+E",
      uiAction: "av-effects",
      mode: "player-focused",
    });
  });

  it("stays disjoint from the shell keymap (no silently shadowed ids)", () => {
    for (const id of DIALOG_KEYMAP.keys()) {
      expect(DEFAULT_KEYMAP.has(id)).toBe(false);
      expect(PLAYER_SHELL_KEYMAP.has(id)).toBe(false);
    }
  });
});

describe("matchModifierBindings (issue #32 — exact-combo matching)", () => {
  const ctrl = (key: string) => ({
    key,
    metaKey: false,
    ctrlKey: true,
    altKey: false,
    shiftKey: false,
  });

  it("matches the exact combo: Ctrl+T fires jump-to-time", () => {
    const hits = matchModifierBindings(DIALOG_KEYMAP, ctrl("t"));
    expect(hits.map((b) => b.id)).toEqual(["jump-to-time"]);
  });

  it("never fires on extra modifiers (Ctrl+Shift+T is not Ctrl+T)", () => {
    expect(
      matchModifierBindings(DIALOG_KEYMAP, { ...ctrl("t"), shiftKey: true }),
    ).toEqual([]);
  });

  it("never fires on the bare key or wrong modifier (Meta+T); Ctrl+B now opens bookmarks", () => {
    expect(
      matchModifierBindings(DIALOG_KEYMAP, {
        key: "t",
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual([]);
    expect(
      matchModifierBindings(DIALOG_KEYMAP, { ...ctrl("t"), metaKey: true, ctrlKey: false }),
    ).toEqual([]);
    expect(matchModifierBindings(DIALOG_KEYMAP, ctrl("b")).map((b) => b.id)).toEqual([
      "bookmarks-dialog",
    ]);
  });

  it("matches the exact combo: Ctrl+E fires effects-and-filters (w26.d1)", () => {
    expect(matchModifierBindings(DIALOG_KEYMAP, ctrl("e")).map((b) => b.id)).toEqual([
      "effects-and-filters",
    ]);
    // extra modifiers never fire the combo
    expect(
      matchModifierBindings(DIALOG_KEYMAP, { ...ctrl("e"), shiftKey: true }),
    ).toEqual([]);
  });

  it("ignores plain-key bindings and non-player-focused modes", () => {
    // A plain binding in the same map is not a combo candidate…
    const mixed = parseKeymap(
      `[x]
combo = { key = "Ctrl+K", ui_action = "k", mode = "player-focused" }
plain = { key = "k", ui_action = "k2", mode = "player-focused" }
other = { key = "Ctrl+J", ui_action = "j", mode = "always" }
`,
    );
    const plainHit = matchModifierBindings(mixed, ctrl("k"));
    expect(plainHit.map((b) => b.id)).toEqual(["combo"]);
    const modeHit = matchModifierBindings(mixed, ctrl("j"));
    expect(modeHit).toEqual([]);
  });
});
