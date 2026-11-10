// Playwright config for the browser-matrix lane (card 7.I1, issue
// #49; doc 09 §8.2/§11.2). Kept separate from playwright.config.ts
// (the e2e smoke lane) and playwright.visual.config.ts — the same
// precedent those two set for each other: this lane does NOT use the
// vite dev server. Its harness server (static files + the engine
// wasm + Range-aware fixture endpoints + per-mode COOP/COEP) is
// started by scripts/web-matrix-run.mjs, which also passes the
// mode→URL map via WEB_MATRIX_BASE_URLS — so this config declares no
// webServer and no baseURL on purpose: a stray `npx playwright test
// --config playwright.web.config.ts` fails fast with the spec's
// "run via npm run webmatrix" error instead of silently testing the
// wrong origin.
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e/web",
  testMatch: "web-matrix.spec.ts",
  timeout: 240_000,
  // Sequential: both isolation modes share the harness server pair
  // and the fragment output dir; parallel workers would only add
  // scheduler noise to a conformance measurement.
  workers: 1,
  fullyParallel: false,
  reporter: [["list"]],
  use: {
    browserName: "chromium",
    headless: true,
  },
});
