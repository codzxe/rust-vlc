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
// OnboardingState seam the visual harness uses, visual.spec.ts:53-61).
//
// w26.k19 (j19 J19-5): the first-run gate had strong UNIT coverage
// (App.test.tsx — 13-route redirect matrix, bypass semantics) but ZERO
// committed browser-level coverage; the "62-route / 68-check" flow bar in
// logs/w26.i20.md §4 was an ad-hoc session battery. The two tests below
// convert it into regression protection — a browser-only break (e.g. store
// hydration vs provider mount order) now fails the committed suite.

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

test("first-run gate: a fresh profile is redirected from any shell route into the wizard", async ({ page }) => {
  // NO seed — a fresh profile. Any AppShell route must land in the wizard
  // (w26.i20: the gate wraps the whole shell); /settings/global is the
  // representative deep link (a browser-only gate regression — e.g. the
  // onboarding store hydrating after the router commits — previously
  // passed every committed e2e).
  await page.goto("/settings/global");
  await expect(page).toHaveURL(/\/onboarding$/);
  await expect(page.getByTestId("onboarding-root")).toBeVisible();
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
});

test("wizard completion: Next ×7 + Get started lands on the player and opens the gate", async ({ page }) => {
  // The honest completion flow i20 drove live (8 steps: welcome → library →
  // appearance → playback → subtitles → privacy → shortcuts → done) — now a
  // committed regression: the wizard must complete in a real browser, land
  // on /player, and persist the exact completeOnboarding() store shape
  // (onboardingState.ts) so the NEXT param-less navigation stays ungated.
  await page.goto("/onboarding");
  for (let step = 0; step < 7; step += 1) {
    await page.getByTestId("onboarding-next").click();
  }
  await page.getByTestId("onboarding-get-started").click();
  await expect(page).toHaveURL(/\/player$/);
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Player");
  const store = await page.evaluate(() => window.localStorage.getItem("lumen.onboarding.v1"));
  expect(JSON.parse(store ?? "null")).toEqual({
    complete: true,
    choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
  });
});
