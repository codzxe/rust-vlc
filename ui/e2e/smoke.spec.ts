import { expect, test } from "@playwright/test";

// Smoke: the dev shell boots and the routing skeleton lands on the player
// placeholder. Green 2026-10-23 (4.A1 closure) on chromium 156.0.8078.4;
// fresh clones need `npx playwright install chromium` once.
test("boot lands on the player placeholder", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Player");
});
