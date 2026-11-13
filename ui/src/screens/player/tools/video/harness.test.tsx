// w26.c4 — harness test: the dev-only vision harness renders all four video
// tools (frame layout) without crashing; with W26C4_HARNESS_OUT set it
// additionally writes standalone HTML snapshots (CSS inlined, the HDR list
// opened) for headless screenshotting in the contract §5 comparison loop
// (the w26.c1/c2 pattern; see the lane log for the loop record).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ComponentType } from "react";
import {
  ColorsHarness,
  CropHarness,
  EssentialHarness,
  HdrHarness,
} from "./harness";

afterEach(cleanup);

/** Styles inlined into the exported HTML: tokens, base sheet, then the
 * component sheets the video tools pull in (the dynamics grammar carries
 * the shared card + slider classes; menu.css the Dropdown; button.css the
 * Toggle). */
const CSS_FILES = [
  "src/tokens/generated/tokens.css",
  "src/theme/lumen.css",
  "src/components/button/button.css",
  "src/components/icon/icon.css",
  "src/components/menu/menu.css",
  "src/components/slider/slider.css",
  "src/screens/player/tools/dynamics/dynamics.css",
  "src/screens/player/tools/video/video.css",
  "src/screens/player/tools/video/harness.css",
];

const HARNESS_STATES: {
  name: string;
  Component: ComponentType;
  /** Accessible names to click before serializing (open-list shots). */
  open?: string[];
}[] = [
  { name: "colors", Component: ColorsHarness },
  { name: "crop", Component: CropHarness },
  { name: "essential", Component: EssentialHarness },
  { name: "hdr-open", Component: HdrHarness, open: ["HDR mode"] },
];

describe("w26.c4 vision harness (dev-only)", () => {
  it("renders all four tools in frame layout without crashing", () => {
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
      cleanup();
    }
  });

  it("exports standalone HTML snapshots when W26C4_HARNESS_OUT is set", () => {
    const out = process.env.W26C4_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
    const uiRoot = process.cwd(); // vitest runs from ui/ (npm run test -- …)
    const css = CSS_FILES.map((file) => {
      const path = resolve(uiRoot, file);
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
    }).join("\n");
    mkdirSync(out, { recursive: true });
    for (const { name, Component, open } of HARNESS_STATES) {
      const view = render(<Component />);
      if (open !== undefined) {
        // Open the requested dropdowns (the SECOND HDR trigger — the first
        // stays closed as the frame's state 1) before serializing.
        const triggers = screen.getAllByRole("button", { name: open[0] });
        const last = triggers[triggers.length - 1];
        if (last !== undefined) fireEvent.click(last);
      }
      const html = [
        "<!doctype html>",
        '<html lang="en"><head><meta charset="utf-8">',
        `<title>w26.c4 harness — ${name}</title>`,
        `<style>${css}</style>`,
        "</head><body>",
        view.container.innerHTML,
        "</body></html>",
      ].join("");
      writeFileSync(resolve(out, `${name}.html`), html);
      cleanup();
    }
    expect(existsSync(resolve(out, "colors.html"))).toBe(true);
    expect(existsSync(resolve(out, "hdr-open.html"))).toBe(true);
  });
});

describe("w26.k11 frame-contract pins (video.css)", () => {
  // Regression pins for the lane's load-bearing composition rules (j11
  // P3-2) — the sheet carries frame-measured geometry and vision-caught
  // fixes, so the CSS itself is pinned (the dynamics harness precedent,
  // dynamics/harness.test.tsx:59-110): a "cleanup" that silently reverts a
  // fix now fails here instead of shipping.
  const css = readFileSync(
    resolve(process.cwd(), "src/screens/player/tools/video/video.css"),
    "utf8",
  );

  it("slider rows stay label-only — the shared value span is scoped away (frame)", () => {
    // "Video - Colors.svg" / "Video - Essential.svg" carry no value column;
    // the span stays for the DOM contract, hidden inside .video-tool.
    expect(css).toMatch(
      /\.video-tool \.dynamics-row__value\s*\{[^}]*display: none;/s,
    );
  });

  it("Crop value fields size to their digits (the vision-caught Bottom fix)", () => {
    // w26.i08: a fixed 3ch reservation ellipsized the "Bottom" label the
    // frame fits; field-sizing: content tracks the drawn digits, capped at
    // the frame's 3-digit trailing reserve.
    expect(css).toMatch(
      /\.video-crop-input__control\s*\{[^}]*field-sizing: content;/s,
    );
    expect(css).toMatch(/\.video-crop-input__control\s*\{[^}]*min-width: 1ch;/s);
    expect(css).toMatch(/\.video-crop-input__control\s*\{[^}]*max-width: 3ch;/s);
  });

  it("the capped-list grammar pins the roomier side + the cap var (i08 escape)", () => {
    // Last-resort cap: data-list-direction pins the side, the cap var sizes
    // the internal scroll (video.css owns the rules the hook sets vars for).
    expect(css).toContain(
      ".video-hdr[data-list-direction] .lumen-dropdown__list",
    );
    expect(css).toContain(
      ".video-subdropdown[data-list-direction] .lumen-dropdown__list",
    );
    expect(css).toMatch(/max-height: var\(--video-dropdown-cap, 320px\);/);
    expect(css).toContain('.video-subdropdown[data-list-direction="up"]');
  });

  it("trigger value alignment is owned by the shared menu.css grammar (j11 P3-4)", () => {
    // w26.i16 landed `text-align: start` on the primitive's trigger
    // (menu.css:316 names this sheet) — video.css must not carry a scoped
    // copy of that declaration (both redundant ones were deleted in w26.k11;
    // the crop input's own `text-align: right` is a different rule — the
    // frame's value column right-alignment — and stays). Asserted against
    // the comment-stripped sheet: the provenance comments cite the shared
    // declaration by name on purpose.
    const declarations = css.replace(/\/\*[\s\S]*?\*\//g, "");
    expect(declarations).not.toContain("text-align: start");
    expect(declarations).not.toMatch(
      /\.lumen-dropdown__trigger[^{]*\{[^}]*text-align/s,
    );
  });
});
