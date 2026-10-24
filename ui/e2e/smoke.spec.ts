import { expect, test } from "@playwright/test";

// Smoke: the dev shell boots and the routing skeleton lands on the player
// placeholder. Green 2026-10-23 (4.A1 closure) on chromium 156.0.8078.4;
// fresh clones need `npx playwright install chromium` once.
//
// Updated by P4-GATE (phase-4 gate, 4.I1-V): the test went stale when 4.F1's
// onboarding flow landed — a FRESH profile (empty localStorage) now boots to
// the first-run wizard ("Welcome to Lumen"), so `/` no longer shows the Player
// heading until the wizard is complete. The 4.A1 smoke expectation is
// preserved by seeding the wizard-complete state first (same
// OnboardingState seam the visual harness uses, visual.spec.ts:53-61);
// onboarding's own first-run behavior is covered by its component tests and
// the visual /onboarding baseline.
test("boot lands on the player placeholder", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "lumen.onboarding.v1",
      JSON.stringify({
        complete: true,
        choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
      }),
    );
  });
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Player");
});
