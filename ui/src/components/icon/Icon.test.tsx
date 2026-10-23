import { render } from "@testing-library/react";
// @ts-expect-error node:fs is untyped in ui/ (no @types/node; zero-new-deps rule, card 4.B1) — vitest runs on Node.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { Icon, iconNames } from "./index";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const iconCss = readFileSync(`${dirname}/icon.css`, "utf8");

const cssRule = (selector: string): string => {
  const match = iconCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
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
