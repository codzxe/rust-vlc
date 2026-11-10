// Browser-matrix spec (card 7.I1, issue #49; doc 09 §8.2/§11.2).
//
// Drives the in-page conformance harness (e2e/web/site/harness.js)
// against the REAL engine Worker in Playwright Chromium, once per
// isolation mode (COOP/COEP off + on — doc 09 §2.5 item 6 adapted to
// the v1 single-threaded truth: both snapshots must stay
// self-consistent and report `threading: "single"`).
//
// This spec is NOT part of `npm run test:e2e` (playwright.config.ts
// excludes e2e/web/**): it needs the matrix harness server, which
// only `npm run webmatrix` (scripts/web-matrix-run.mjs) starts —
// that driver owns the config-driven ports, the artifact staging and
// the report merge (docs/web/browser-matrix-*.json).
//
// Firefox/WebKit lanes are NOT launched here: no Firefox/WebKit
// browsers exist in this sandbox (download forbidden), so those lanes
// are recorded as BCD-cited expectations by the report builder —
// "not runnable in this sandbox", never a fabricated result.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "@playwright/test";

interface ScenarioResult {
  name: string;
  category: string;
  status: "pass" | "fail";
  ms: number;
  evidence?: unknown;
  error?: string;
}

interface SuiteResult {
  scenarios: ScenarioResult[];
  probes: {
    crossOriginIsolated: boolean;
    isolationMode: string;
  };
}

interface WasmResponseRecord {
  url: string;
  status: number;
  contentType: string | null;
  contentLength: string | null;
}

const here = path.dirname(fileURLToPath(import.meta.url));

interface MatrixConfig {
  server: {
    servedFixture: string;
    rangeSupportingPath: string;
    rangeBlindPath: string;
  };
  modes: Array<{ id: string; coopCoep: boolean }>;
  pump: { ticksPerSlice: number; maxSlices: number; sliceTimeoutMs: number };
  playToEnd: { positionToleranceUs: number };
}

const matrix = JSON.parse(
  readFileSync(path.join(here, "matrix.config.json"), "utf8"),
) as MatrixConfig;

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(
      `${name} is not set — this spec runs via \`npm run webmatrix\` ` +
        "(scripts/web-matrix-run.mjs starts the harness servers and passes it)",
    );
  }
  return value;
}

const baseUrls = JSON.parse(requireEnv("WEB_MATRIX_BASE_URLS")) as Record<string, string>;
const outDir = requireEnv("WEB_MATRIX_OUT_DIR");
const fixtureBytes = Number(requireEnv("WEB_MATRIX_FIXTURE_BYTES"));

for (const mode of matrix.modes) {
  const baseUrl = baseUrls[mode.id];
  if (!baseUrl) {
    throw new Error(`no harness server URL for mode ${mode.id}`);
  }

  test(`chromium lane — ${mode.id}`, async ({ page, browser }) => {
    // Capture the wasm response the WORKER's own fetch received (the
    // MIME evidence for streaming instantiation — doc 09 §2.3).
    const wasmResponses: WasmResponseRecord[] = [];
    page.on("response", (response) => {
      if (response.url().endsWith("lumen_web.wasm")) {
        wasmResponses.push({
          url: response.url(),
          status: response.status(),
          contentType: response.headers()["content-type"] ?? null,
          contentLength: response.headers()["content-length"] ?? null,
        });
      }
    });

    await page.goto(baseUrl);
    const runtime = {
      server: {
        servedFixture: `/${matrix.server.servedFixture}`,
        rangeSupportingPath: `/${matrix.server.rangeSupportingPath}`,
        rangeBlindPath: `/${matrix.server.rangeBlindPath}`,
      },
      pump: matrix.pump,
      playToEnd: matrix.playToEnd,
      mode,
      fixtureBytes,
    };
    const result = await page.evaluate<SuiteResult, unknown>(
      (cfg) =>
        (
          window as unknown as {
            __lumenWebMatrix: { run: (config: unknown) => Promise<SuiteResult> };
          }
        ).__lumenWebMatrix.run(cfg),
      runtime,
    );

    // The isolation truth: the page's crossOriginIsolated must match
    // the served headers, and the capability snapshot stays
    // single-threaded either way (doc 09 §3.2 — v1 ignores isolation).
    const expectedIsolated = mode.coopCoep;
    const observedIsolated = result.probes.crossOriginIsolated;
    if (observedIsolated !== expectedIsolated) {
      throw new Error(
        `crossOriginIsolated mismatch for mode ${mode.id}: expected ` +
          `${expectedIsolated}, observed ${observedIsolated}`,
      );
    }
    // The fragment is written BEFORE the pass/fail verdict: the
    // machine-readable record must carry FAILING scenarios too (a
    // failed run is evidence — the report and the honesty pass
    // consume per-scenario rows either way; writing fragments only
    // on a green run would silently erase red evidence).
    mkdirSync(outDir, { recursive: true });
    const fragment = {
      browser: "chromium",
      browserVersion: browser.version(),
      evidence: "playwright-run",
      mode: mode.id,
      coopCoep: mode.coopCoep,
      crossOriginIsolated: observedIsolated,
      userAgent: await page.evaluate(() => navigator.userAgent),
      wasmResponses,
      suite: result,
    };
    const fragmentPath = path.join(outDir, `chromium-${mode.id}.json`);
    writeFileSync(fragmentPath, `${JSON.stringify(fragment, null, 2)}\n`);

    const failed = result.scenarios.filter((scenario) => scenario.status === "fail");
    if (failed.length > 0) {
      const names = failed.map((scenario) => scenario.name).join(", ");
      throw new Error(`${failed.length} scenario(s) failed: ${names}`);
    }
  });
}
