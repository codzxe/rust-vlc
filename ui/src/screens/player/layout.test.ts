// Card 4.C1 — layout-config parsing tests (doc 07 §7.2 consumption side).
import { describe, expect, it } from "vitest";
import {
  DEFAULT_LAYOUT,
  LayoutParseError,
  evaluateCondition,
  parseLayoutConfig,
  resolveLayout,
} from "./layout";

describe("parseLayoutConfig (doc 07 §7.2)", () => {
  it("parses the default layout with all player-anatomy regions in order", () => {
    expect(DEFAULT_LAYOUT.schema).toBe("layout.v1");
    expect(DEFAULT_LAYOUT.regions.map((r) => r.id)).toEqual([
      "video-canvas",
      "title-bar",
      "controls-bar",
      "library",
      "playlist",
      "media-info",
      "customize",
    ]);
    const byId = new Map(DEFAULT_LAYOUT.regions.map((r) => [r.id, r]));
    expect(byId.get("video-canvas")).toMatchObject({ anchor: "center", order: 10 });
    expect(byId.get("library")).toMatchObject({ anchor: "left", labelKey: "player.region.library" });
    expect(byId.get("media-info")).toMatchObject({ anchor: "right", visibleWhen: "has-media" });
  });

  it("parses the default menu tree with its ui-action leaf", () => {
    expect(DEFAULT_LAYOUT.menus).toHaveLength(1);
    const menu = DEFAULT_LAYOUT.menus[0];
    expect(menu).toBeDefined();
    expect(menu!.id).toBe("app");
    expect(menu!.items).toHaveLength(2);
    expect(menu!.items[0]).toMatchObject({ id: "open-media", uiAction: "open-media" });
    // Card 4.F3 — the customize surface opens via the menu/ui-action seam.
    expect(menu!.items[1]).toMatchObject({ id: "customize-player", uiAction: "open-panel:customize" });
  });

  it("parses a minimal valid custom layout and sorts regions by order", () => {
    const layout = parseLayoutConfig(`schema = "layout.v1"
[[region]]
id = "canvas"
anchor = "center"
label_key = "player.canvas.label"
order = 2
[[region]]
id = "bar"
anchor = "top"
label_key = "player.region.title-bar"
order = 1
`);
    expect(layout.regions.map((r) => r.id)).toEqual(["bar", "canvas"]);
  });

  it("falls back to the default layout only for a missing/empty source", () => {
    expect(resolveLayout(undefined)).toBe(DEFAULT_LAYOUT);
    expect(resolveLayout("   \n ")).toBe(DEFAULT_LAYOUT);
  });

  it("rejects a wrong schema version", () => {
    expect(() => parseLayoutConfig('schema = "layout.v2"')).toThrow(LayoutParseError);
  });

  it("rejects an unknown anchor", () => {
    const src = `schema = "layout.v1"
[[region]]
id = "x"
anchor = "diagonal"
label_key = "player.canvas.label"
`;
    expect(() => parseLayoutConfig(src)).toThrow(/unknown anchor "diagonal"/);
  });

  it("rejects duplicate region ids", () => {
    const src = `schema = "layout.v1"
[[region]]
id = "x"
anchor = "top"
label_key = "player.region.title-bar"
[[region]]
id = "x"
anchor = "bottom"
label_key = "player.region.controls-bar"
`;
    expect(() => parseLayoutConfig(src)).toThrow(/duplicate region id "x"/);
  });

  it("rejects an unknown condition token", () => {
    const src = `schema = "layout.v1"
[[region]]
id = "x"
anchor = "top"
label_key = "player.region.title-bar"
visible_when = "maybe-tuesday"
`;
    expect(() => parseLayoutConfig(src)).toThrow(/unknown condition token "maybe-tuesday"/);
  });

  it("rejects a label_key that does not resolve (doc 07 §6.4 rule 4)", () => {
    const src = `schema = "layout.v1"
[[region]]
id = "x"
anchor = "top"
label_key = "nope.does.not.exist"
`;
    expect(() => parseLayoutConfig(src)).toThrow(/unknown label_key/);
  });

  it("rejects a layout with no regions", () => {
    expect(() => parseLayoutConfig('schema = "layout.v1"')).toThrow(/at least one \[\[region\]\]/);
  });
});

describe("evaluateCondition (doc 07 §7.3 condition tokens)", () => {
  const ctx = { hasMedia: true, hasVideo: false, fullscreen: true, hasChapters: false };
  it("undefined token is always visible", () => {
    expect(evaluateCondition(undefined, ctx)).toBe(true);
  });
  it("evaluates each token against the context", () => {
    expect(evaluateCondition("has-media", ctx)).toBe(true);
    expect(evaluateCondition("has-video", ctx)).toBe(false);
    expect(evaluateCondition("fullscreen", ctx)).toBe(true);
    expect(evaluateCondition("has-chapters", ctx)).toBe(false);
  });
});
