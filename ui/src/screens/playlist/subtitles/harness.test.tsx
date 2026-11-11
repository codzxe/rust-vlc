// w26.b11 — harness test: the dev-only vision harness renders every frame
// state without crashing; with W26B11_HARNESS_OUT set it additionally
// writes standalone HTML snapshots (CSS inlined, dropdown open, candidate
// rows selected) for headless screenshotting in the contract §5 comparison
// loop (see the lane log for the loop record).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ComponentType } from "react";
import {
  FolderMultiHarness,
  FolderOnlineHarness,
  FolderPairsHarness,
  SubFilenameHarness,
  SubtitleRowsHarness,
} from "./harness";

afterEach(cleanup);

/** Styles inlined into the exported HTML: tokens, base sheet, then the
 * component sheets the subtitle flow pulls in. */
const CSS_FILES = [
  "src/tokens/generated/tokens.css",
  "src/theme/lumen.css",
  "src/components/button/button.css",
  "src/components/icon/icon.css",
  "src/components/surface/surface.css",
  "src/components/menu/menu.css",
  "src/components/list/list.css",
  "src/screens/playlist/subtitles/subtitles.css",
  "src/screens/playlist/subtitles/harness.css",
];

interface HarnessState {
  name: string;
  Component: ComponentType;
  /** Row texts to click before serializing (pre-selected frame states). */
  select?: string[];
  /** Accessible trigger name to click before serializing (open dropdowns);
   * `index` picks among same-named triggers (default 0). */
  open?: { name: string; index?: number };
}

const HARNESS_STATES: HarnessState[] = [
  { name: "folder-pairs", Component: FolderPairsHarness },
  { name: "folder-multi", Component: FolderMultiHarness, select: ["The_Wand_S01E01_English.srt", "The_Wand_S01E01_Spanish.srt"] },
  { name: "folder-online", Component: FolderOnlineHarness, select: ["The_Wand_S01E01_English.srt"] },
  { name: "sub-filename", Component: SubFilenameHarness, open: { name: "Filename matching", index: 1 } },
  { name: "subtitle-rows", Component: SubtitleRowsHarness },
];

describe("w26.b11 vision harness (dev-only)", () => {
  it("renders all frame states without crashing", () => {
    for (const { Component } of HARNESS_STATES) {
      const view = render(<Component />);
      expect(view.container.innerHTML.length).toBeGreaterThan(0);
      cleanup();
    }
  });

  it("exports standalone HTML snapshots when W26B11_HARNESS_OUT is set", () => {
    const out = process.env.W26B11_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
    const uiRoot = process.cwd(); // vitest runs from ui/ (npm run test -- …)
    const css = CSS_FILES.map((file) => {
      const path = resolve(uiRoot, file);
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
    }).join("\n");
    mkdirSync(out, { recursive: true });
    for (const { name, Component, select, open } of HARNESS_STATES) {
      cleanup();
      render(<Component />);
      if (open !== undefined) {
        const triggers = screen.getAllByRole("button", { name: open.name });
        fireEvent.click(triggers[open.index ?? 0]!);
      }
      for (const rowText of select ?? []) {
        fireEvent.click(screen.getByText(rowText));
      }
      const body = document.body.innerHTML;
      writeFileSync(
        resolve(out, `${name}.html`),
        `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head><body>${body}</body></html>`,
        "utf8",
      );
      expect(body.length).toBeGreaterThan(0);
    }
  });
});
