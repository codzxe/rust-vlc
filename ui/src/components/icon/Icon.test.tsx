import { render } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { Icon, iconNames, IconWrapper } from "./index";
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
        "next",
        "previous",
        "volume-mute",
        "volume-low",
        "volume-medium",
        "volume-high",
        "fullscreen",
        "menu",
        "close",
        "more",
        "search",
        "sort",
        "list",
        "grid",
        "thumbnail",
        "add",
        "check",
        "chevron-up",
        "chevron-down",
        "chevron-left",
        "chevron-right",
        "folder",
        "person",
        "profile",
        "queue",
        "equalizer",
        "audio",
        "subtitles",
        "info",
        "wand",
        "favorite",
        "record",
        "snapshot",
        "pip",
        "hdr",
        "repeat",
        "shuffle",
        "settings",
      ]),
    );
    // 32 frozen (wave-1) + 53 frame-mined w26.a07 additions (see lane log)
    expect(iconNames.length).toBe(85);
    expect(new Set(iconNames).size).toBe(iconNames.length);
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

  it("pins mined sheet geometry — regression guard against provisional paths (issue #35)", () => {
    // Fingerprints of the translated design/figma/Icon.svg path data (bbox
    // centred on (12,12)). If these change, geometry was re-drawn by hand
    // instead of mined — re-derive from the sheet, don't edit the constants.
    const d = (name: IconName, index: number): string => {
      const { unmount } = render(<Icon name={name} />);
      const paths = [...document.querySelectorAll("svg.lumen-icon path")].map((p) => p.getAttribute("d") ?? "");
      unmount();
      return paths[index] ?? "";
    };
    // play: Icon.svg Vector 16_4 (424→7.75, 80.9554→16.955 after centring)
    expect(d("play", 0).startsWith("M7.75 16.955V7.045C7.75 6.213 8.707 5.744")).toBe(true);
    // settings: Vector 75 gear outline + Ellipse 10 centre hole
    expect(d("settings", 0).startsWith("M12.504 21H11.597")).toBe(true);
    expect(d("settings", 1).startsWith("M14.25 12C14.25 13.243")).toBe(true);
    // volume-mute: misnamed "close, value=none" group — speaker + X cross
    expect(d("volume-mute", 1)).toBe("M16 10L18 12M18 12L20 14M18 12L20 10M18 12L16 14");
    // skip-back-10: backward-10-sec arrow (reverse-curled) + outlined "10"
    expect(d("skip-back-10", 0).startsWith("M21 10.44C20.571 8.44")).toBe(true);
    expect(d("skip-back-10", 1).startsWith("M11.459 12.167V19.44")).toBe(true);
    // open-file: page mask with 4-radius TL corner (Rectangle 1 in menu.svg row)
    expect(d("open-file", 0).startsWith("M3 8C3 4.686 5.686 2 9 2")).toBe(true);
  });

  it("pins w26.a07 mined geometry — next/previous/menu/search/close/check/more/chevron", () => {
    const d = (name: IconName, index: number): string => {
      const { unmount } = render(<Icon name={name} />);
      const paths = [...document.querySelectorAll("svg.lumen-icon path")].map((p) => p.getAttribute("d") ?? "");
      unmount();
      return paths[index] ?? "";
    };
    // next: sheet name=next (Frame 17) = Controls.svg Frame 10 transport
    expect(d("next", 0)).toBe("M7.5 17V7L13.5 12L7.5 17Z");
    expect(d("next", 1)).toBe("M14.5 17V7H16.5V17H14.5Z");
    // previous: sheet name=previous (Frame 18) = Controls.svg Frame 8 transport
    expect(d("previous", 0)).toBe("M7.5 17V7H9.5V17H7.5Z");
    expect(d("previous", 1)).toBe("M16.5 17V7L10.5 12L16.5 17Z");
    // menu: Controls.svg hamburger — nub + line, three rows at 5u pitch
    expect(d("menu", 0)).toBe("M4 7H5");
    expect(d("menu", 1)).toBe("M8 7H20");
    expect(d("menu", 5)).toBe("M8 17H20");
    // search: magnifier circle + handle (Frame 1_19)
    expect(d("search", 0).startsWith("M15.345 15.345C16.495 14.195")).toBe(true);
    // close: the ✕ (Frame 34)
    expect(d("close", 0)).toBe("M7 7L17 17");
    // check: Frame 1_15 = option.svg Vector 44
    expect(d("check", 0)).toBe("M5 12L9.667 17.5L19 6.5");
    // more: three dots (Frame 49)
    expect(d("more", 0)).toBe("M6 12L6 12");
    // chevrons (Frame_88/73/90/89)
    expect(d("chevron-down", 0)).toBe("M7 9.5L12 14.5L17 9.5");
    expect(d("chevron-up", 0)).toBe("M7 14.5L12 9.5L17 14.5");
  });

  it("registry aliases render the identical mined glyph (folder/person/info/wand)", () => {
    const geometry = (name: IconName): string => {
      const { unmount } = render(<Icon name={name} />);
      const svg = document.querySelector("svg.lumen-icon")!;
      const serial = svg.innerHTML;
      unmount();
      return serial;
    };
    expect(geometry("folder")).toBe(geometry("open-folder"));
    expect(geometry("person")).toBe(geometry("profile"));
    expect(geometry("info")).toBe(geometry("about"));
    expect(geometry("wand")).toBe(geometry("customize-player"));
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

describe("IconWrapper (w26.a07, design/figma/Icon Wrapper.svg)", () => {
  it("frames an icon with the default md hit area (30×32 frame)", () => {
    const { unmount } = render(
      <IconWrapper>
        <Icon name="settings" />
      </IconWrapper>,
    );
    const wrapper = document.querySelector(".lumen-icon-wrapper")!;
    expect(wrapper).toBeTruthy();
    expect(wrapper.getAttribute("class")).toContain("lumen-icon-wrapper--md");
    expect(wrapper.querySelector("svg.lumen-icon")).toBeTruthy();
    unmount();
  });

  it("exposes sm/md/lg size presets; lg is the ≥44px rail target", () => {
    for (const size of ["sm", "md", "lg"] as const) {
      const { unmount } = render(
        <IconWrapper size={size}>
          <Icon name="go-home" />
        </IconWrapper>,
      );
      expect(document.querySelector(`.lumen-icon-wrapper--${size}`)).toBeTruthy();
      unmount();
    }
    expect(cssRule(".lumen-icon-wrapper--sm")).toContain("24px");
    expect(cssRule(".lumen-icon-wrapper--md")).toContain("30px");
    expect(cssRule(".lumen-icon-wrapper--md")).toContain("32px");
    expect(cssRule(".lumen-icon-wrapper--lg")).toContain("44px");
  });

  it("active state flags the orange accent (token-driven, no raw colors)", () => {
    const { unmount } = render(
      <IconWrapper active>
        <Icon name="go-home" />
      </IconWrapper>,
    );
    expect(document.querySelector(".lumen-icon-wrapper--active")).toBeTruthy();
    unmount();
    expect(cssRule(".lumen-icon-wrapper--active .lumen-icon")).toContain("var(--color-accent-icon)");
  });

  it("hover applies the subtle framed backdrop from Icon Wrapper.svg via tokens", () => {
    expect(cssRule(".lumen-icon-wrapper")).toContain("4px"); // frame: Icon Wrapper.svg rx=4
    expect(cssRule(".lumen-icon-wrapper:hover")).toContain("var(--color-surface-hover)");
    // token discipline: no raw colors in declarations (comments quote frame hexes)
    const declarationsOnly = iconCss.replace(/\/\*[\s\S]*?\*\//g, "");
    expect(declarationsOnly).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
  });
});
