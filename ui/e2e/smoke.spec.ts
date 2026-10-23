import { expect, test } from "@playwright/test";

// Smoke: the dev shell boots and the routing skeleton lands on the player
// placeholder. Requires installed browsers (`npx playwright install
// chromium`) — deferred, see NEXT-STEPS.md.
test("boot lands on the player placeholder", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Player");
});
