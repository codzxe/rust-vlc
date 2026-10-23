import { defineConfig } from "@playwright/test";

// E2E smoke config (doc 08 §9.3/§9.6 will grow the matrix). Browsers ARE
// installed now (chromium 156.0.8078.4, 4.A1 closure session) — fresh clones
// still run `npx playwright install chromium` once before `npm run test:e2e`.
export default defineConfig({
  testDir: "./e2e",
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
