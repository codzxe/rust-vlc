// Playwright config for the screenshot-diff harness (card 4.G1, doc 08
// §6.3). Kept separate from playwright.config.ts so the e2e smoke/theme
// suite (and its reporter/output layout) is untouched; both reuse the same
// dev server on 5173 (reuseExistingServer).
//
// Baseline layout: e2e/visual/baselines/<platform>/<case-id>.png — per-OS
// baseline sets are the doc 08 §6.3 accepted reality (font rasterization
// differs); Linux is the canonical set (committed by 4.G1/G2).
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e/visual",
  timeout: 30_000,
  fullyParallel: true,
  // Doc 08 §6.4 flakiness control: exactly one retry — the whole test
  // (including the screenshot) re-runs, separating a transient diff from a
  // real one. A real regression fails twice.
  retries: 1,
  reporter: [
    ["list"],
    // Under playwright-report/ so the existing .gitignore rule covers it.
    ["html", { open: "never", outputFolder: "playwright-report/visual" }],
  ],
  // The tolerance mechanism (doc 08 §6.2): Playwright's built-in pixel
  // comparison. Values + rationale documented in visual.spec.ts and
  // ui/README.md §Visual screenshot-diff harness.
  snapshotPathTemplate: "e2e/visual/baselines/{platform}/{arg}{ext}",
  expect: {
    toHaveScreenshot: {
      // Per-pixel perceptual (YIQ) delta under which a pixel counts as
      // unchanged. 0.1 ≈ invisible AA jitter; tighter than the 0.2 default
      // to catch token/color drift.
      threshold: 0.1,
      // Doc 08 §6.2's ≤0.1%-differing-pixels pass threshold, applied
      // globally (Playwright has no tolerance zones) [PROPOSAL].
      maxDiffPixelRatio: 0.001,
      // Flakiness: rewind/disable CSS animations + hide the caret.
      animations: "disabled",
      caret: "hide",
      scale: "css",
    },
  },
  use: {
    browserName: "chromium",
    headless: true,
    // Host-parity determinism (wave-L round 3, CI visual run 43384519508):
    // the committed Inter face removed the FONT-FILE variable (every host
    // loads the same woff2), but glyph RASTERIZATION still drifted between
    // the pinning host's FreeType and the hosted runner's (ubuntu-latest
    // ships a different build; 19/30 PR cases failed at ~0.18% differing
    // pixels — text-edge AA jitter, vs the 0.1% bar). Chromium honors
    // fontconfig hinting from the host, which is the cross-host variance
    // source; `--font-render-hinting=none` disables it at the browser
    // level so glyph outlines rasterize from the font file alone — same
    // font bytes + same chromium build + no hinting = same pixels on any
    // Linux host. Baselines are pinned under this exact flag (the 5th
    // re-pin; MATRIX-20261113.md records the diagnosis).
    launchOptions: {
      args: ["--font-render-hinting=none"],
    },
    // 127.0.0.1, not localhost — contract §9 (the dev server binds 127.0.0.1
    // explicitly; w26.k20, j20 P3-13).
    baseURL: "http://127.0.0.1:5173",
  },
  webServer: {
    command: "npm run dev -- --port 5173 --strictPort",
    url: "http://127.0.0.1:5173",
    // Reuse the already-running dev server locally (the Lead-owned 5173
    // server during waves), never in CI — a stale CI server must fail, not
    // be silently reused (Playwright's documented !process.env.CI pattern;
    // w26.k20, j20 P3-13).
    reuseExistingServer: !process.env.CI,
    // 240s budget — same rationale as playwright.config.ts (run 43370495290:
    // the 60s webServer default killed the visual PR-lite lane on cold CI
    // runners before a single case executed).
    timeout: 240_000,
  },
});
