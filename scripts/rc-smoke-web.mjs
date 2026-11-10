#!/usr/bin/env node
// rc-smoke-web.mjs — browser-load half of the RC smoke harness (card 8.G2,
// issue #58). scripts/rc-smoke.sh builds ui/ (npm run build), serves the
// production bundle with `vite preview`, then runs THIS script.
//
// What it proves, for real, in a chromium browser:
//   * the built SPA loads over HTTP (no dev server, no HMR),
//   * a fresh profile boots through the first-run onboarding gate to the
//     player surface (the same OnboardingState seam ui/e2e/smoke.spec.ts
//     seeds — the wizard itself is covered by its component tests),
//   * the app reports ZERO uncaught page errors while doing so.
//
// Playwright pin drift, honestly: ui/package-lock.json pins @playwright/test
// 1.64.1, which expects chromium revision 1248; reference sandboxes ship
// chromium-1200 (Chrome for Testing 143) and never download browsers. The
// harness therefore launches the DISCOVERED chromium through an explicit
// executablePath (passed by rc-smoke.sh) instead of playwright's pinned
// revision lookup. That bridge is exercised by the smoke run; if the
// revision gap ever grows past what the CDP protocol tolerates, this script
// fails loudly here first (docs/guides/troubleshooting.md, "Playwright says
// the browser executable doesn't exist").
//
// Usage: node rc-smoke-web.mjs <origin-url> <chromium-executable>
// Exit: 0 pass, 1 fail, 2 usage/preflight.

import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import path from "node:path";

const [, , origin, chromiumPath] = process.argv;
if (!origin || !chromiumPath) {
  console.error("usage: rc-smoke-web.mjs <origin-url> <chromium-executable>");
  process.exit(2);
}

// Resolve @playwright/test from the ui/ workspace (this script lives in
// scripts/, outside ui/, so bare-specifier resolution would miss it).
const uiPackageJson = path.resolve(import.meta.dirname, "..", "ui", "package.json");
const require = createRequire(pathToFileURL(uiPackageJson));
let chromium;
try {
  ({ chromium } = require("@playwright/test"));
} catch (err) {
  console.error(`rc-smoke-web: cannot load @playwright/test from ui/ (${err.message}) — run npm ci in ui/ first`);
  process.exit(2);
}

const pageErrors = [];
let browser;
try {
  browser = await chromium.launch({
    headless: true,
    executablePath: chromiumPath,
    args: ["--no-sandbox"],
  });
  const page = await browser.newPage();
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Seed the wizard-complete state BEFORE any app script runs (same seam as
  // ui/e2e/smoke.spec.ts).
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "lumen.onboarding.v1",
      JSON.stringify({
        complete: true,
        choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
      }),
    );
  });

  await page.goto(origin, { waitUntil: "load", timeout: 20_000 });
  // The player surface renders its level-1 heading; give the SPA a beat.
  const heading = await page.waitForSelector("h1", { timeout: 10_000 });
  const headingText = (await heading.textContent())?.trim() ?? "";
  if (!headingText) {
    throw new Error("boot check failed: the app rendered no level-1 heading");
  }
  console.log(`rc-smoke-web: loaded ${origin}`);
  console.log(`rc-smoke-web: level-1 heading: ${JSON.stringify(headingText)}`);
  console.log(`rc-smoke-web: chromium: ${browser.version()}`);

  if (pageErrors.length > 0) {
    for (const err of pageErrors) console.error(`rc-smoke-web: pageerror: ${err}`);
    throw new Error(`boot check failed: ${pageErrors.length} uncaught page error(s)`);
  }
  console.log("rc-smoke-web: PASS (loaded, booted to the app surface, 0 page errors)");
  await browser.close();
  process.exit(0);
} catch (err) {
  console.error(`rc-smoke-web: FAILED: ${err.message}`);
  await browser?.close().catch(() => {});
  process.exit(1);
}
