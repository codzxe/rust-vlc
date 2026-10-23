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
    baseURL: "http://localhost:5173", // pairs with webServer.url below
  },
  webServer: {
    command: "npm run dev -- --port 5173 --strictPort",
    url: "http://localhost:5173",
    reuseExistingServer: true,
  },
});
