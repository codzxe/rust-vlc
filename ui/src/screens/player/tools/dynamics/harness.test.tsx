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

describe("w26.i07 frame-contract pins (dynamics.css)", () => {
  // Regression pins for the h14 fixes — the CSS carries the frame-measured
  // geometry, so the sheet itself is pinned (the states.css/tokens.css
  // test precedent).
  const css = readFileSync(
    resolve(process.cwd(), "src/screens/player/tools/dynamics/dynamics.css"),
    "utf8",
  );

  it("thumb right edge sits flush with the fill end (h14 defect 2)", () => {
    // The wave-f L8 flush travel, applied to the tool scope that used to
    // override the primitive with the overhanging (100% - 8px) formula.
    expect(css).toContain(
      "left: calc((100% - 4px) * var(--lumen-slider-thumb, 0) / 100 - 6px);",
    );
    expect(css).not.toContain(
      "left: calc((100% - 8px) * var(--lumen-slider-thumb, 0) / 100);",
    );
  });

  it("card chevron renders at native scale with the frame right inset (h14 defect 3)", () => {
    // Icon size 24 (DynamicsCard) + the −7.5px calibration → stroke 18.25px
    // off the card edge (frame Vector 27 stroke right x361.75 on x20–380).
    expect(css).toContain("margin-right: -7.5px;");
  });

  it("degree ticks paint the frame's warm sw2 round-capped marks", () => {
    // 2×8 painted pill; ink ≈ frame #1A1611 composed from semantic tokens.
    expect(css).toMatch(/\.dynamics-slider__tick\s*\{[^}]*height: 8px;/s);
    expect(css).toMatch(
      /color-mix\(\s*in srgb,\s*var\(--color-surface-muted\) 83%,\s*var\(--color-accent-tinted\)/,
    );
  });

  it("light theme remaps wells through semantic tokens (h14 defect 1)", () => {
    expect(css).toContain(':root[data-theme="light"] .dynamics-tool__toggle');
    expect(css).toContain(
      ':root[data-theme="light"] .dynamics-slider .lumen-slider__track',
    );
    // Wells map to the ramp's recessed step, never raw hex.
    expect(css.match(/--color-surface-deep/g)?.length).toBeGreaterThanOrEqual(3);
    // Quiet ink keeps its role in BOTH themes: content-tertiary, which the
    // token lane (i19 / h20 F1) repinned to an AA-passing light value
    // (#474747 — 5.44:1 on the light panel). No light ink override exists.
    expect(css).toMatch(
      /\.dynamics-row__label\s*\{[^}]*color: var\(--color-content-tertiary\);/s,
    );
    expect(css).not.toMatch(
      /:root\[data-theme="light"\][^{]*\.dynamics-row__(?:label|value)/,
    );
  });
});
