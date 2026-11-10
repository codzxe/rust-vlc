import { defineConfig } from "@playwright/test";

// E2E smoke config (doc 08 §9.3/§9.6 will grow the matrix). Browsers ARE
// installed now (chromium 156.0.8078.4, 4.A1 closure session) — fresh clones
// still run `npx playwright install chromium` once before `npm run test:e2e`.
export default defineConfig({
  testDir: "./e2e",
  // snapshotDir trap (4.H1 handoff, docs/gates/phase4-exit.md §4 item 6):
  // without this, the plain `npx playwright test` (npm run test:e2e) also
  // picks up e2e/visual/visual.spec.ts and runs its PR cases against
  // Playwright's DEFAULT snapshot location (…-snapshots/ under e2e/visual/)
  // instead of the committed baselines/linux/ set — "snapshot doesn't exist,
  // writing actual" x30, spurious red + stray snapshot dirs. The visual
  // harness is owned by playwright.visual.config.ts and driven exclusively
  // via npm run visual / scripts/visual-run.mjs; exclude it here so the
  // plain command cannot mislead.
  // e2e/web/** is excluded for the same reason (card 7.I1): the browser
  // matrix needs its own harness server (engine wasm + Range endpoints +
  // per-mode COOP/COEP), started only by npm run webmatrix —
  // playwright.web.config.ts owns that lane.
  testIgnore: ["**/visual/**", "**/web/**"],
  timeout: 30_000,
  fullyParallel: true,
  // Explicit reporters: list for terminal, HTML (never auto-open) for the
  // CI artifact upload in .github/workflows/ui.yml (4.A1 proposal).
  reporter: [
    ["list"],
    ["html", { open: "never", outputFolder: "playwright-report" }],
  ],
  use: {
    browserName: "chromium",
    headless: true,
    // Relative page.goto("/") targets resolve against the dev server
    // (card 4.A2: without an explicit baseURL, goto("/") is an invalid URL —
    // latent 4.A1 gap, first exercised now that browsers are installed).
    baseURL: "http://localhost:5173",
  },
  webServer: {
    command: "npm run dev -- --port 5173 --strictPort",
    url: "http://localhost:5173",
    reuseExistingServer: true,
  },
});
