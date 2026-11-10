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
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";

// The chromium launch + runner knobs (matrix.config.json → browsers.chromium
// + runner): the driver resolves the executable (env override → config path
// under the Playwright browsers root → @playwright/test's own revision-pinned
// binary) and hands it over via the configured env name; the launch flags and
// the per-test timeout come straight from the config (the container-evidence
// shape: --no-sandbox + --disable-dev-shm-usage — the config's comment
// explains why every lane passes them; no literal here, doc 07 §9 — W24-R8
// P3-2 moved the runner timeout under the same rule).
interface MatrixBrowserConfig {
  browsers: {
    chromium: {
      executable: string | null;
      executableEnv: string;
      launchArgs: string[];
    };
  };
  runner: {
    timeoutMs: number;
  };
}
const here = path.dirname(fileURLToPath(import.meta.url));
const matrix = JSON.parse(
  readFileSync(path.join(here, "e2e", "web", "matrix.config.json"), "utf8"),
) as MatrixBrowserConfig;
const chromiumExecutable = process.env[matrix.browsers.chromium.executableEnv];
const chromiumLaunchArgs = matrix.browsers.chromium.launchArgs ?? [];
if (!Number.isInteger(matrix.runner.timeoutMs) || matrix.runner.timeoutMs <= 0) {
  throw new Error(
    "matrix.config.json: runner.timeoutMs must be a positive integer " +
      `(got ${JSON.stringify(matrix.runner.timeoutMs)}) — the driver validates the same knob`,
  );
}

export default defineConfig({
  testDir: "./e2e/web",
  testMatch: "web-matrix.spec.ts",
  timeout: matrix.runner.timeoutMs,
  // Sequential: both isolation modes share the harness server pair
  // and the fragment output dir; parallel workers would only add
  // scheduler noise to a conformance measurement.
  workers: 1,
  fullyParallel: false,
  reporter: [["list"]],
  use: {
    browserName: "chromium",
    headless: true,
    launchOptions: {
      ...(chromiumExecutable ? { executablePath: chromiumExecutable } : {}),
      ...(chromiumLaunchArgs.length > 0 ? { args: chromiumLaunchArgs } : {}),
    },
  },
});
