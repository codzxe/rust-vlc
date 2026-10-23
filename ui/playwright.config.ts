import { defineConfig } from "@playwright/test";

// E2E smoke config (doc 08 §9.3/§9.6 will grow the matrix). Browsers are NOT
// installed yet — run `npx playwright install chromium` before `npm run
// test:e2e` (deferred from card 4.A1 session 1, see NEXT-STEPS.md).
export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  fullyParallel: true,
  use: {
    browserName: "chromium",
    headless: true,
  },
  webServer: {
    command: "npm run dev -- --port 5173 --strictPort",
    url: "http://localhost:5173",
    reuseExistingServer: true,
  },
});
