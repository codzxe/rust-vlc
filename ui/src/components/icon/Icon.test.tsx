import { render } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { Icon, iconNames } from "./index";
import type { IconName } from "./icons";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const iconCss = readFileSync(`${dirname}/icon.css`, "utf8");

const cssRule = (selector: string): string => {
  const match = iconCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
};

/** All numeric coordinates of an element's geometry (d / cx / cy / r / x / y …). */
const coordNumbers = (el: Element): number[] => {
  const nums: number[] = [];
  for (const attr of ["d", "cx", "cy", "r", "rx", "ry", "x", "y", "width", "height"]) {
    const v = el.getAttribute(attr);
    if (v) nums.push(...(v.match(/-?\d+(?:\.\d+)?/g) ?? []).map(Number));
  }
  return nums;
};

describe("Icon (1.5-stroke set, doc 08 §1.4/§1.5)", () => {
  it("renders an svg with the 1.5 stroke, currentColor, and the token class", () => {
    render(<Icon name="play" />);
    const svg = document.querySelector("svg.lumen-icon")!;
    expect(svg).toBeTruthy();
    expect(svg.getAttribute("stroke")).toBe("currentColor");
    expect(svg.getAttribute("stroke-width")).toBe("1.5");
    expect(svg.getAttribute("viewBox")).toBe("0 0 24 24");
    expect(svg.getAttribute("class")).toContain("lumen-icon");
    expect(svg.querySelectorAll("path, rect, circle, text").length).toBeGreaterThan(0);
  });

  it("every registry name renders a glyph", () => {
    for (const name of iconNames) {
      const { unmount } = render(<Icon name={name} />);
      const svg = document.querySelector("svg.lumen-icon")!;
      expect(svg.querySelectorAll("*").length, `glyph missing for ${name}`).toBeGreaterThan(0);
      unmount();
    }
  });

  it("registry holds exactly the doc-named icons (no invented icons)", () => {
    expect(iconNames).toEqual(
      expect.arrayContaining([
        "play",
        "pause",
        "stop",
        "volume-mute",
        "volume-low",
        "volume-medium",
        "volume-high",
        "record",
        "snapshot",
        "pip",
        "hdr",
        "repeat",
        "shuffle",
        "settings",
      ]),
    );
    expect(iconNames.length).toBe(32);
  });

  it("size prop maps to width/height", () => {
    const { unmount } = render(<Icon name="zoom" size={24} />);
    const svg = document.querySelector("svg.lumen-icon")!;
    expect(svg.getAttribute("width")).toBe("24");
    expect(svg.getAttribute("height")).toBe("24");
    unmount();
  });

  it("is aria-hidden when decorative and role=img when titled", () => {
    const { unmount } = render(<Icon name="stop" />);
    let svg = document.querySelector("svg.lumen-icon")!;
    expect(svg.getAttribute("aria-hidden")).toBe("true");
    expect(svg.getAttribute("role")).toBeNull();
    unmount();

    render(<Icon name="stop" title="Stop playback" />);
    svg = document.querySelector("svg.lumen-icon")!;
    expect(svg.getAttribute("role")).toBe("img");
    expect(svg.getAttribute("aria-label")).toBe("Stop playback");
    expect(svg.querySelector("title")!.textContent).toBe("Stop playback");
  });

  it("disabled state applies the §4.4 class; CSS carries token state variants", () => {
    const { unmount } = render(<Icon name="quit" disabled />);
    expect(document.querySelector("svg.lumen-icon")!.getAttribute("class")).toContain("lumen-icon--disabled");
    unmount();

    expect(cssRule(".lumen-icon")).toContain("var(--color-stroke-icon)");
    expect(cssRule(".lumen-icon-host:hover .lumen-icon")).toContain("var(--color-content-bright)");
    expect(cssRule(".lumen-icon-host--selected .lumen-icon")).toContain("var(--color-accent-icon)");
    expect(cssRule(".lumen-icon--disabled")).toContain("var(--opacity-disabled)");
  });
});

describe("Icon mined glyph geometry (issue #35, design/figma/Icon.svg)", () => {
  it("keeps every glyph inside the 24×24 grid (translated, not rescaled)", () => {
    for (const name of iconNames) {
      const { unmount } = render(<Icon name={name} />);
      const svg = document.querySelector("svg.lumen-icon")!;
      for (const el of svg.querySelectorAll("path, rect, circle, ellipse")) {
        for (const n of coordNumbers(el)) {
          // 1u slack absorbs stroke overhang + digit outlines that kiss the edge
          expect(n, `${name}: coordinate ${n} escapes the 24×24 grid`).toBeGreaterThanOrEqual(-1);
          expect(n, `${name}: coordinate ${n} escapes the 24×24 grid`).toBeLessThanOrEqual(25);
        }
      }
      unmount();
    }
  });

  it("transport glyphs are filled like the design (play.svg type=play/pause, Icon.svg stop)", () => {
    const transport: IconName[] = ["play", "pause", "stop"];
    for (const name of transport) {
      const { unmount } = render(<Icon name={name} />);
      const svg = document.querySelector("svg.lumen-icon")!;
      const filled = [...svg.querySelectorAll("path")].filter((p) => p.getAttribute("fill") === "currentColor");
      expect(filled.length, `${name} should render filled geometry`).toBeGreaterThan(0);
      unmount();
    }
  });

  it("volume family shares the mined speaker and adds one wave per level", () => {
    // Each variant is centred on its own bbox exactly like the design sheet,
    // so absolute coordinates shift per level — compare the command/argument
    // STRUCTURE of the speaker (numbers abstracted) instead of raw coords.
    const structure = (d: string) => d.replace(/-?\d+(?:\.\d+)?/g, "#");
    const glyph = (name: IconName) => {
      const { unmount } = render(<Icon name={name} />);
      const svg = document.querySelector("svg.lumen-icon")!;
      const paths = [...svg.querySelectorAll("path")].map((p) => p.getAttribute("d") ?? "");
      unmount();
      return paths;
    };
    const speaker = structure(glyph("volume-high")[0] ?? "");
    const expected: Record<string, number> = {
      "volume-mute": 1, // the X cross
      "volume-low": 1,
      "volume-medium": 2,
      "volume-high": 3,
    };
    for (const [name, waves] of Object.entries(expected)) {
      const paths = glyph(name as IconName);
      expect(paths.length, `${name} should be speaker + ${waves} marks`).toBe(1 + waves);
      expect(structure(paths[0] ?? ""), `${name} should use the shared speaker shape`).toBe(speaker);
      for (const mark of paths.slice(1)) {
        expect(structure(mark), `${name} marks should be strokes/cross bars`).toMatch(/^M# #L# #/);
      }
    }
  });

  it("per-element stroke-width overrides from the sheet survive (info stem/dot at 2)", () => {
    const { unmount } = render(<Icon name="about" />);
    const svg = document.querySelector("svg.lumen-icon")!;
    const sw2 = [...svg.querySelectorAll("path")].filter((p) => p.getAttribute("stroke-width") === "2");
    expect(sw2.length).toBe(2); // stem + dot
    unmount();
  });
});

describe("Icon emphasis variants (issue #35, doc 08 §1.4 emphasis strokes)", () => {
  it("default variant keeps the 1.5 stroke and no modifier class", () => {
    const { unmount } = render(<Icon name="settings" />);
    const svg = document.querySelector("svg.lumen-icon")!;
    expect(svg.getAttribute("stroke-width")).toBe("1.5");
    expect(svg.getAttribute("class")).not.toContain("lumen-icon--strong");
    expect(svg.getAttribute("class")).not.toContain("lumen-icon--max");
    unmount();
  });

  it("strong variant switches to the 2px emphasis stroke (--stroke-icon-strong)", () => {
    const { unmount } = render(<Icon name="settings" variant="strong" />);
    const svg = document.querySelector("svg.lumen-icon")!;
    expect(svg.getAttribute("stroke-width")).toBe("2");
    expect(svg.getAttribute("class")).toContain("lumen-icon--strong");
    unmount();
  });

  it("max variant switches to the 3px emphasis stroke (--stroke-icon-max)", () => {
    const { unmount } = render(<Icon name="repeat" variant="max" />);
    const svg = document.querySelector("svg.lumen-icon")!;
    expect(svg.getAttribute("stroke-width")).toBe("3");
    expect(svg.getAttribute("class")).toContain("lumen-icon--max");
    unmount();
  });

  it("variant does not change the a11y contract", () => {
    const { unmount } = render(<Icon name="stop" variant="strong" />);
    expect(document.querySelector("svg.lumen-icon")!.getAttribute("aria-hidden")).toBe("true");
    unmount();
    render(<Icon name="stop" variant="strong" title="Stop" />);
    expect(document.querySelector("svg.lumen-icon")!.getAttribute("role")).toBe("img");
    unmount();
  });
});
