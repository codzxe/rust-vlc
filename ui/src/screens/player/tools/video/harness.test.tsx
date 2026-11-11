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
