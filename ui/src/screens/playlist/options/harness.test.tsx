// w26.b10 — harness test: the dev-only vision harness renders every frame
// state without crashing; with W26B10_HARNESS_OUT set it additionally
// writes standalone HTML snapshots (CSS inlined, menus opened) for headless
// screenshotting in the contract §5 comparison loop (see the lane log for
// the loop record).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ComponentType } from "react";
import { OptionsMenusHarness, OptionsViewsHarness, OptionsPopupCloseups } from "./harness";

afterEach(cleanup);

/** Styles inlined into the exported HTML: tokens, base sheet, then the
 * component sheets the options layer pulls in. */
const CSS_FILES = [
  "src/tokens/generated/tokens.css",
  "src/theme/lumen.css",
  "src/components/button/button.css",
  "src/components/icon/icon.css",
  "src/components/list/list.css",
  "src/components/menu/menu.css",
  "src/screens/playlist/options/options.css",
  "src/screens/playlist/options/harness.css",
];

interface HarnessState {
  name: string;
  Component: ComponentType;
  /** Trigger accessible names to click before serializing (menu-open shots). */
  open?: string[];
}

const HARNESS_STATES: HarnessState[] = [
  { name: "views", Component: OptionsViewsHarness },
  { name: "sorting-open", Component: OptionsPopupCloseups, open: ["Sorting options"] },
  { name: "view-open", Component: OptionsPopupCloseups, open: ["View options"] },
];

describe("w26.b10 vision harness (dev-only)", () => {
  it("renders all frame states without crashing", () => {
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
      cleanup();
    }
    const menus = render(<OptionsMenusHarness />);
    expect(menus.container.innerHTML.length).toBeGreaterThan(0);
  });

  it("exports standalone HTML snapshots when W26B10_HARNESS_OUT is set", () => {
    const out = process.env.W26B10_HARNESS_OUT;
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
        // Open the requested anchored menus before serializing.
        for (const label of open) fireEvent.click(screen.getByRole("button", { name: label }));
      }
      const html = [
        "<!doctype html>",
        '<html lang="en"><head><meta charset="utf-8">',
        `<title>w26.b10 harness — ${name}</title>`,
        `<style>${css}</style>`,
        "</head><body>",
        view.container.innerHTML,
        "</body></html>",
      ].join("");
      writeFileSync(resolve(out, `${name}.html`), html);
      cleanup();
    }
    expect(existsSync(resolve(out, "sorting-open.html"))).toBe(true);
    expect(existsSync(resolve(out, "view-open.html"))).toBe(true);
    expect(existsSync(resolve(out, "views.html"))).toBe(true);
  });
});
