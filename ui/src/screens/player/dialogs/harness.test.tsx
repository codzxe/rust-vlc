// w26.c5 — harness test: the dev-only vision harness renders the mounted
// ToolsDialog (both tab variants) without crashing; with W26C5_HARNESS_OUT
// set it additionally writes standalone HTML snapshots (CSS inlined) for
// headless screenshotting in the contract §5 comparison loop (the w26.c2
// harness pattern).

import { cleanup, render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { ToolsDialogHarness } from "./harness";

/** Styles inlined into the exported HTML: tokens, base sheet, then the
 * component sheets the dialog + tools pull in. */
const CSS_FILES = [
  "src/tokens/generated/tokens.css",
  "src/theme/lumen.css",
  "src/components/surface/surface.css",
  "src/components/button/button.css",
  "src/components/icon/icon.css",
  "src/components/form/form.css",
  "src/components/menu/menu.css",
  "src/components/slider/slider.css",
  "src/components/list/list.css",
  "src/screens/player/tools/equalizer/equalizer.css",
  "src/screens/player/tools/dynamics/dynamics.css",
  "src/screens/player/tools/visualizer/visualizer.css",
  "src/screens/player/tools/video/video.css",
  "src/screens/player/dialogs/dialogs.css",
];

describe("w26.c5 vision harness (dev-only)", () => {
  it("renders the tools dialog on both tabs without crashing", () => {
    for (const tab of ["audio", "video"] as const) {
      const view = render(<ToolsDialogHarness tab={tab} />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
      // The dialog chrome + tab pair + every tool card mount.
      expect(view.getAllByRole("tab")).toHaveLength(2);
      cleanup();
    }
  });

  // w26.k09 (j09 P3): skipIf makes the env-gated export EXPLICIT in the
  // runner output instead of an always-passing no-op — the fs/inline-CSS
  // path runs only when W26C5_HARNESS_OUT is set (dev-only tooling).
  it.skipIf(
    process.env.W26C5_HARNESS_OUT === undefined || process.env.W26C5_HARNESS_OUT === "",
  )("exports standalone HTML snapshots when W26C5_HARNESS_OUT is set", () => {
    const out = process.env.W26C5_HARNESS_OUT as string;
    const uiRoot = process.cwd(); // vitest runs from ui/ (npm run test -- …)
    const css = CSS_FILES.map((file) => {
      const path = resolve(uiRoot, file);
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
    }).join("\n");
    mkdirSync(out, { recursive: true });
    for (const tab of ["audio", "video"] as const) {
      const view = render(<ToolsDialogHarness tab={tab} />);
      writeFileSync(
        resolve(out, `tools-dialog-${tab}.html`),
        `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head><body style="margin:0;background:var(--color-surface-deep);min-height:100vh">${view.container.innerHTML}</body></html>`,
      );
      cleanup();
    }
  });
});
