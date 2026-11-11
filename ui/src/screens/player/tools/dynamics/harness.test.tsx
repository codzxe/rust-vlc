// w26.c2 — harness test: the dev-only vision harness renders the three
// dynamics tools (frame layout) without crashing; with W26C2_HARNESS_OUT set
// it additionally writes a standalone HTML snapshot (CSS inlined) for
// headless screenshotting in the contract §5 comparison loop (see the lane
// log for the loop record).

import { cleanup, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { DynamicsToolsHarness } from "./harness";

/** Styles inlined into the exported HTML: tokens, base sheet, then the
 * component sheets the dynamics tools pull in. */
const CSS_FILES = [
  "src/tokens/generated/tokens.css",
  "src/theme/lumen.css",
  "src/components/icon/icon.css",
  "src/components/slider/slider.css",
  "src/screens/player/tools/dynamics/dynamics.css",
  "src/screens/player/tools/dynamics/harness.css",
];

describe("w26.c2 vision harness (dev-only)", () => {
  it("renders all three tools in frame layout without crashing", () => {
    const view = render(<DynamicsToolsHarness />);
    expect(view.container.innerHTML.length).toBeGreaterThan(0);
    // The harness never ships collapsed-only: every tool also mounts its
    // expanded state for the comparison shot.
    expect(view.container.querySelectorAll(".dynamics-tool")).toHaveLength(6);
    cleanup();
  });

  it("exports a standalone HTML snapshot when W26C2_HARNESS_OUT is set", () => {
    const out = process.env.W26C2_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
    const uiRoot = process.cwd(); // vitest runs from ui/ (npm run test -- …)
    const css = CSS_FILES.map((file) => {
      const path = resolve(uiRoot, file);
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
    }).join("\n");
    mkdirSync(out, { recursive: true });
    const view = render(<DynamicsToolsHarness />);
    const html = [
      "<!doctype html>",
      '<html lang="en"><head><meta charset="utf-8">',
      "<title>w26.c2 harness — dynamics tools</title>",
      `<style>${css}</style>`,
      "</head><body>",
      view.container.innerHTML,
      "</body></html>",
    ].join("");
    writeFileSync(resolve(out, "dynamics-tools.html"), html);
    cleanup();
    expect(existsSync(resolve(out, "dynamics-tools.html"))).toBe(true);
  });
});
