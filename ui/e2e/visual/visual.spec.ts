// Screenshot-diff fidelity harness (card 4.G1, doc 08 §6).
//
// Walks the screen manifest (manifest.ts), renders each case under
// Playwright Chromium with deterministic-rendering controls, and diffs the
// clipped #root area against the pinned baselines in
// e2e/visual/baselines/<platform>/ via expect(page).toHaveScreenshot —
// Playwright's built-in pixel comparison IS the tolerance mechanism
// (doc 08 §6.2's pixelmatch-class diff; zero new npm deps).
//
// Modes (set by scripts/visual-run.mjs via VISUAL_MODE):
//   VISUAL_MODE=pr       PR-lite lane: 1280×720 @1×, dark + light (fast, strict)
//   VISUAL_MODE=nightly  full matrix: viewports × DPI × modes
//
// Seeded-regression proof: SEED_2PX=1 injects a test-only 2-px shift of
// the app shell; the harness must FAIL. Never set outside the demo/tests.
//
// Flakiness controls (doc 08 §6.4): prefers-reduced-motion forced,
// CSS animations disabled at screenshot time, network-idle + font-ready +
// settle delay before capture, clip pinned to #root (stable area), and
// retries=1 in playwright.visual.config.ts (a transient diff gets one
// fresh re-screenshot; a real regression fails twice).

import { expect, test } from "@playwright/test";
import { MANIFEST, caseId } from "./manifest";

// 4.F4: zero-dep node-env typing (repo rule — no @types/node; the generated
// settings test documents the same seam). Restores `tsc --noEmit` over e2e/.
declare const process: { env: Record<string, string | undefined> };

const MODE = process.env.VISUAL_MODE === "nightly" ? "nightly" : "pr";
const SEED_2PX = process.env.SEED_2PX === "1";
const cases = MANIFEST.filter((c) => MODE === "nightly" || c.pr);

for (const c of cases) {
  const id = caseId(c);

  test.describe(`visual ${id}`, () => {
    // DPI cannot change after context creation, so it rides on test.use.
    test.use({
      viewport: { width: c.width, height: c.height },
      deviceScaleFactor: c.dpi,
    });

    test(`matches pinned baseline (${MODE})`, async ({ page }) => {
      // Doc 08 §6.4: animations off. colorScheme pinned dark so the
      // provider's "auto" fallback resolves deterministically; the actual
      // mode/accent are forced below via the attribute seam (ThemeProvider
      // applies data-theme/data-accent on <html>; setting them post-mount
      // is stable because the effect only re-runs on state change).
      await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
      // Deterministic app state (doc 08 §6.4 "same stubbed state"): mark the
      // first-run wizard complete for every route except the wizard itself,
      // so /player etc. render their real screens, not the onboarding
      // redirect. Matches OnboardingState (screens/onboarding/onboardingState.ts).
      if (c.route !== "/onboarding") {
        await page.addInitScript(() => {
          window.localStorage.setItem(
            "lumen.onboarding.v1",
            JSON.stringify({
              complete: true,
              choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
            }),
          );
        });
      }
      await page.goto(c.route, { waitUntil: "networkidle" });

      await page.evaluate(
        async ({ mode, accent }) => {
          document.documentElement.dataset.theme = mode;
          document.documentElement.dataset.accent = accent;
          await document.fonts.ready;
        },
        { mode: c.theme, accent: c.accent },
      );
      // Settle: token cascade repaint + one paint frame.
      await page.waitForTimeout(250);

      // Stable screenshot area: clip to the app root (card 4.G1), so
      // viewport chrome / scrollbars never enter the diff.
      const box = await page.locator("#root").boundingBox();
      if (box === null) {
        throw new Error("#root has no bounding box — cannot clip a stable area");
      }

      if (SEED_2PX) {
        // Test-only regression seed (AC proof): shift the app shell 2 px.
        // transform, not margin, so the clip rect itself is unaffected —
        // the diff must come from moved content, not a moved window.
        await page.addStyleTag({
          content: "#root > * { transform: translateX(2px) !important; }",
        });
        await page.waitForTimeout(50);
      }

      // Tolerance: threshold 0.1 per-pixel (perceptual YIQ delta; below
      // ~0.1 is imperceptible AA jitter — tighter than Playwright's 0.2
      // default to catch token/color drift) + maxDiffPixelRatio 0.001
      // (doc 08 §6.2's ≤0.1%-differing-pixels threshold, applied globally
      // because Playwright has no zone concept [PROPOSAL]). 2-px shifts of
      // viewport-scale elements produce ~0.3%+ differing pixels → FAIL.
      await expect(page).toHaveScreenshot(`${id}.png`, { clip: box });
    });
  });
}
