// w26.c3 — harness test: the dev-only vision harness renders the
// visualizer selector's frame states + the tool card states without
// crashing; with W26C3_HARNESS_OUT set it additionally writes standalone
// HTML snapshots (CSS inlined, the frame cell's SECOND selector opened)
// for headless screenshotting in the contract §5 comparison loop (the
// w26.c1/c2 pattern; see the lane log for the loop record).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ComponentType } from "react";
import {
  VisualizerSelectorStates,
  VisualizerToolStates,
} from "./harness";

afterEach(cleanup);

/** Styles inlined into the exported HTML: tokens, base sheet, then the
 * component sheets the visualizer tool pulls in (menu.css since w26.k10 —
 * the selector is the shared Dropdown now, and its grammar lives there). */
const CSS_FILES = [
  "src/tokens/generated/tokens.css",
  "src/theme/lumen.css",
  "src/components/icon/icon.css",
  "src/components/menu/menu.css",
  "src/screens/player/tools/visualizer/visualizer.css",
  "src/screens/player/tools/visualizer/harness.css",
];

interface HarnessState {
  name: string;
  Component: ComponentType;
  /** Open the frame cell's SECOND selector (the open-list state). */
  openSecondSelector?: boolean;
}

const HARNESS_STATES: HarnessState[] = [
  { name: "selector-states", Component: VisualizerSelectorStates, openSecondSelector: true },
  { name: "tool-card", Component: VisualizerToolStates },
];

describe("w26.c3 vision harness (dev-only)", () => {
  it("renders both harness compositions without crashing", () => {
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
      cleanup();
    }
  });

  it("exports standalone HTML snapshots when W26C3_HARNESS_OUT is set", () => {
    const out = process.env.W26C3_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
    const uiRoot = process.cwd(); // vitest runs from ui/ (npm run test -- …)
    const css = CSS_FILES.map((file) => {
      const path = resolve(uiRoot, file);
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
    }).join("\n");
    mkdirSync(out, { recursive: true });
    for (const { name, Component, openSecondSelector } of HARNESS_STATES) {
      const view = render(<Component />);
      if (openSecondSelector === true) {
        // The frame cell's SECOND selector is the open-list state.
        const triggers = screen.getAllByRole("button", { name: "Visualizer" });
        const openTrigger = triggers[1];
        if (openTrigger === undefined) throw new Error("second selector trigger missing");
        fireEvent.click(openTrigger);
      }
      const html = [
        "<!doctype html>",
        '<html lang="en"><head><meta charset="utf-8">',
        `<title>w26.c3 harness — ${name}</title>`,
        `<style>${css}</style>`,
        "</head><body>",
        view.container.innerHTML,
        "</body></html>",
      ].join("");
      writeFileSync(resolve(out, `${name}.html`), html);
      cleanup();
    }
    expect(existsSync(resolve(out, "selector-states.html"))).toBe(true);
    expect(existsSync(resolve(out, "tool-card.html"))).toBe(true);
  });
});

describe("w26.i07 frame-contract pins (visualizer.css)", () => {
  // Regression pins for the h14 fixes — the sheet carries the frame-measured
  // geometry + the light-theme remap (dynamics.css pins the shared grammar).
  const css = readFileSync(
    resolve(process.cwd(), "src/screens/player/tools/visualizer/visualizer.css"),
    "utf8",
  );

  it("card chevron carries the family's frame right inset (h14 defect 3)", () => {
    // Icon size 24 (VisualizerTool) + the −7.5px calibration → stroke
    // 18.25px off the card edge (tool-card family grammar, Vector 27).
    expect(css).toMatch(
      /\.viz-tool__chevron\s*\{[^}]*margin-right: -7\.5px;/s,
    );
  });

  it("light theme remaps wells through semantic tokens (h14 defect 1)", () => {
    // Collapsed card / display well / trigger ring → the recessed step.
    // Quiet ink keeps its role (content-tertiary — i19/h20 F1 repinned the
    // light value to AA-passing #474747); no light ink override exists.
    // w26.k10: the trigger ring rule now targets the composed shared
    // Dropdown trigger (border = the 2px ring).
    expect(css).toContain(':root[data-theme="light"] .viz-tool {');
    expect(css).toContain(':root[data-theme="light"] .viz-display');
    expect(css).toContain(
      ':root[data-theme="light"] .viz-select .lumen-dropdown__trigger',
    );
    expect(css).toMatch(
      /\.viz-tool__toggle\s*\{[^}]*color: var\(--color-content-tertiary\);/s,
    );
    expect(css).not.toMatch(
      /:root\[data-theme="light"\][^{]*\.viz-tool__(?:toggle|chevron)\s*\{/,
    );
  });

  it("composes the shared Dropdown for the mode selector (j10 P2-4)", () => {
    // The lane-scoped combobox grammar is gone: only the A7 tool-scale
    // scopes remain, keyed on the primitive's own classes.
    expect(css).toContain(".viz-select .lumen-dropdown__trigger {");
    expect(css).toContain(".viz-select .lumen-dropdown__option {");
    expect(css).not.toMatch(/\.viz-select__(?:trigger|field|list|option|value|chevron|check|icon)/);
  });
});
