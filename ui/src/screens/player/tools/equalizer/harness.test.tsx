// w26.c1 — harness test: the dev-only vision harness renders every frame
// state without crashing; with W26C1_HARNESS_OUT set it additionally writes
// standalone HTML snapshots (CSS inlined, preset list opened) for headless
// screenshotting in the contract §5 comparison loop (the w26.b10 pattern;
// see the lane log for the loop record).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ComponentType } from "react";
import { EqualizerCardStates, EqualizerPresetsHarness } from "./harness";

afterEach(cleanup);

/** Styles inlined into the exported HTML: tokens, base sheet, then the
 * component sheets the equalizer tool pulls in. */
const CSS_FILES = [
  "src/tokens/generated/tokens.css",
  "src/theme/lumen.css",
  "src/components/form/form.css",
  "src/components/icon/icon.css",
  "src/components/menu/menu.css",
  "src/components/slider/slider.css",
  "src/screens/player/tools/equalizer/equalizer.css",
  "src/screens/player/tools/equalizer/harness.css",
];

interface HarnessState {
  name: string;
  Component: ComponentType;
  /** Accessible names to click before serializing (menu-open shots). */
  open?: string[];
}

const HARNESS_STATES: HarnessState[] = [
  { name: "card", Component: EqualizerCardStates },
  { name: "presets-open", Component: EqualizerPresetsHarness, open: ["Equalizer preset"] },
];

describe("w26.c1 vision harness (dev-only)", () => {
  it("renders all frame states without crashing", () => {
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
      cleanup();
    }
  });

  it("exports standalone HTML snapshots when W26C1_HARNESS_OUT is set", () => {
    const out = process.env.W26C1_HARNESS_OUT;
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
        // Open the requested dropdowns before serializing.
        for (const label of open) {
          fireEvent.click(screen.getByRole("button", { name: label }));
        }
      }
      const html = [
        "<!doctype html>",
        '<html lang="en"><head><meta charset="utf-8">',
        `<title>w26.c1 harness — ${name}</title>`,
        `<style>${css}</style>`,
        "</head><body>",
        view.container.innerHTML,
        "</body></html>",
      ].join("");
      writeFileSync(resolve(out, `${name}.html`), html);
      cleanup();
    }
    expect(existsSync(resolve(out, "card.html"))).toBe(true);
    expect(existsSync(resolve(out, "presets-open.html"))).toBe(true);
  });
});

describe("w26.k10 frame-contract pins (equalizer.css)", () => {
  // Regression pins in the dynamics/visualizer harness-test precedent
  // (states.css string pins) for the EQ sheet's load-bearing composition
  // rules — the g15 rail-pan fix had NO pin (j10 P3-8) and the preset
  // glyphs keyed on DOM position until w26.k10 (j10 P3-6).
  const css = readFileSync(
    resolve(process.cwd(), "src/screens/player/tools/equalizer/equalizer.css"),
    "utf8",
  );

  it("the fader rail shrinks to its mount and pans (g15 P1-1 / j10 P3-8)", () => {
    // width 100% + max-width 748 + overflow-x: the 748px frame rail pans
    // inside narrow hosts (the 398px tools dialog) instead of overflowing
    // under the scrim; every column keeps the frame geometry.
    expect(css).toMatch(/\.eq-panel__rail\s*\{[^}]*width: 100%;/s);
    expect(css).toMatch(/\.eq-panel__rail\s*\{[^}]*max-width: 748px;/s);
    expect(css).toMatch(/\.eq-panel__rail\s*\{[^}]*overflow-x: auto;/s);
    expect(css).toMatch(/\.eq-panel__rail\s*\{[^}]*scrollbar-width: thin;/s);
  });

  it("preset curve glyphs key on the structural data-preset marker (j10 P3-6)", () => {
    expect(css).toContain(
      '.eq-tool__presets .lumen-dropdown__option[data-preset="dance"]::before',
    );
    expect(css).toContain(
      '.eq-tool__presets .lumen-dropdown__option[data-preset="full-treble"]::before',
    );
    // Never DOM position again — the pre-k10 :nth-child keying was only as
    // safe as the option order.
    expect(css).not.toMatch(/:nth-child/);
  });
});
