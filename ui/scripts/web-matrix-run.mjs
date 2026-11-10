#!/usr/bin/env node
// Browser-matrix driver (card 7.I1, issue #49; doc 09 §8.2/§11.2).
//
// Stages the harness site (the committed e2e/web/site page + the
// engine wasm artifact + the worker glue + the first-vertical
// fixture), starts one static server per isolation mode (config
// driven — matrix.config.json owns the port, the paths and the mode
 // list; the COOP/COEP pair is served per mode so both capability
// snapshots are exercised, doc 09 §2.5 item 6), drives Playwright
// Chromium through the in-page conformance suite, and merges the
// per-mode fragments into the machine-readable matrix record
// (docs/web/browser-matrix-<date>.json).
//
// Firefox/WebKit/Safari lanes are NEVER fabricated: their report rows
// are BCD-cited expectations + "not runnable in this sandbox" + the
// next step (doc 09 §8.2's manual protocol), composed below.
//
// Zero new npm deps (node:*, like the rest of ui/scripts). Browsers
// are never downloaded (PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1): the
// sandbox's preinstalled Chromium is used via its default cache path.
//
// Usage: node scripts/web-matrix-run.mjs
// Exit code: Playwright's, propagated verbatim (visual-run.mjs rule).

import { createHash } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { createReadStream } from "node:fs";
import { copyFileSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const pkgDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(pkgDir, "..");
const configPath = path.join(pkgDir, "e2e", "web", "matrix.config.json");
const config = JSON.parse(readFileSync(configPath, "utf8"));

// The config is the single source of every knob; a malformed one must
// fail LOUDLY here, not surface as a silently-wrong harness run (a
// duplicate JSON key would otherwise let the wrong "browsers" table win).
function validateConfigShape(subject) {
  const required = [
    ["server", "host"],
    ["server", "port"],
    ["server", "targetDirs"],
    ["server", "wasmArtifact"],
    ["server", "wasmArtifactFallback"],
    ["server", "workerGlue"],
    ["server", "workletGlue"],
    ["server", "fixture"],
    ["server", "servedFixture"],
    ["server", "rangeSupportingPath"],
    ["server", "rangeBlindPath"],
    ["browsers", "chromium", "executable"],
    ["browsers", "chromium", "executableEnv"],
    ["browsers", "chromium", "launchArgs"],
    ["browsers", "chromium", "evidence"],
    ["pump", "ticksPerSlice"],
    ["pump", "maxSlices"],
    ["pump", "sliceTimeoutMs"],
    ["playToEnd", "positionToleranceUs"],
    ["report", "outputDir"],
    ["report", "markdownName"],
    ["report", "jsonName"],
  ];
  for (const pathKeys of required) {
    let node = subject;
    let missing = false;
    for (const key of pathKeys) {
      if (node === undefined || node[key] === undefined) {
        missing = true;
        break;
      }
      node = node[key];
    }
    if (missing) {
      fail(`matrix.config.json is missing ${pathKeys.join(".")}`);
    }
  }
  if (!Array.isArray(subject.modes) || subject.modes.length === 0) {
    fail("matrix.config.json: modes must be a non-empty list");
  }
  for (const mode of subject.modes) {
    if (typeof mode.id !== "string" || typeof mode.coopCoep !== "boolean") {
      fail(`matrix.config.json: bad mode entry ${JSON.stringify(mode)}`);
    }
  }
  if (!Array.isArray(subject.browsers.chromium.launchArgs)) {
    fail("matrix.config.json: browsers.chromium.launchArgs must be a list");
  }
  for (const flag of subject.browsers.chromium.launchArgs) {
    if (typeof flag !== "string" || !flag.startsWith("--")) {
      fail(`matrix.config.json: bad launch flag ${JSON.stringify(flag)}`);
    }
  }
}
validateConfigShape(config);

const siteSrc = path.join(pkgDir, "e2e", "web", "site");
const siteDist = path.join(pkgDir, "e2e", "web", "sitedist");

function fail(message) {
  console.error(`web-matrix: ${message}`);
  process.exit(2);
}

function resolveRepo(relative) {
  return path.join(repoRoot, relative);
}

// -- stage the site ---------------------------------------------------------

// -- locate the wasm artifact -----------------------------------------------

function firstExisting(candidates) {
  for (const candidate of candidates) {
    try {
      statSync(candidate);
      return candidate;
    } catch {
      // try the next
    }
  }
  return null;
}

// CARGO_TARGET_DIR wins (the parallel-worktree build environment exports
// it); otherwise the config's candidate list, each resolved against the
// repo root — the worktree-local `target/` first, then any other layout
// the config names. No path is hardcoded here (doc 07 §9's rule).
const targetDirs = [
  ...(process.env.CARGO_TARGET_DIR ? [process.env.CARGO_TARGET_DIR] : []),
  ...config.server.targetDirs.map((relative) => resolveRepo(relative)),
];
const wasmCandidates = targetDirs.flatMap((dir) => [
  path.join(dir, ...config.server.wasmArtifact.split("/")),
  path.join(dir, ...config.server.wasmArtifactFallback.split("/")),
]);
const wasmPath = firstExisting(wasmCandidates);
if (!wasmPath) {
  fail(
    "no lumen_web.wasm artifact (looked in: " +
      wasmCandidates.join(", ") +
      ") — build it first: " +
      "cargo build -p lumen-web --target wasm32-unknown-unknown --locked",
  );
}
const wasmProfile = wasmPath.includes(`${path.sep}release${path.sep}`) ? "release" : "debug";

const fixturePath = resolveRepo(config.server.fixture);
const workerPath = resolveRepo(config.server.workerGlue);
const workletPath = resolveRepo(config.server.workletGlue);
const fixtureBytes = statSync(fixturePath).size;
const wasmBytes = statSync(wasmPath).size;
const wasmSha = createHash("sha256").update(readFileSync(wasmPath)).digest("hex");

mkdirSync(siteDist, { recursive: true });
copyFileSync(path.join(siteSrc, "index.html"), path.join(siteDist, "index.html"));
copyFileSync(path.join(siteSrc, "harness.js"), path.join(siteDist, "harness.js"));
copyFileSync(workerPath, path.join(siteDist, "worker.js"));
copyFileSync(workletPath, path.join(siteDist, "audio-worklet.js"));
copyFileSync(wasmPath, path.join(siteDist, "lumen_web.wasm"));
copyFileSync(fixturePath, path.join(siteDist, config.server.servedFixture));

// -- the static server (one per isolation mode) ------------------------------

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".avi": "video/x-msvideo",
};

/** Parse `bytes=a-b` / `bytes=a-` (end-inclusive, RFC 9110 §14). */
function parseRange(header, total) {
  if (!header || !header.startsWith("bytes=")) {
    return null;
  }
  const spec = header.slice("bytes=".length);
  const dash = spec.indexOf("-");
  if (dash < 1) {
    return null;
  }
  const start = Number.parseInt(spec.slice(0, dash), 10);
  if (!Number.isInteger(start) || start < 0 || start >= total) {
    return null;
  }
  const endText = spec.slice(dash + 1);
  let endInclusive;
  if (endText === "") {
    endInclusive = total - 1;
  } else {
    endInclusive = Number.parseInt(endText, 10);
    if (!Number.isInteger(endInclusive) || endInclusive < start) {
      return null;
    }
    endInclusive = Math.min(endInclusive, total - 1);
  }
  return { start, endInclusive };
}

function startServer(mode, port) {
  const server = http.createServer((request, response) => {
    const url = new URL(request.url ?? "/", "http://invalid.local");
    let route = url.pathname;
    if (route === "/") {
      route = "/index.html";
    }
    const filePath = path.normalize(path.join(siteDist, route));
    if (!filePath.startsWith(siteDist)) {
      response.writeHead(403);
      response.end("forbidden");
      return;
    }
    let stats;
    try {
      stats = statSync(filePath);
    } catch {
      response.writeHead(404);
      response.end("not found");
      return;
    }
    const headers = {
      "content-type": MIME[path.extname(filePath)] ?? "application/octet-stream",
    };
    if (mode.coopCoep) {
      // doc 09 §3.3/§9.5: the isolation pair, served per mode.
      headers["cross-origin-opener-policy"] = "same-origin";
      headers["cross-origin-embedder-policy"] = "require-corp";
    }
    // The range-blind route: the unseekable-transport shape the 7.H4
    // seekability classification consumes (no Accept-Ranges, 200 full
    // body even when a Range arrives).
    const rangeBlind = route === `/${config.server.rangeBlindPath}`;
    const rangeHeader = request.headers.range;
    const range = rangeBlind ? null : parseRange(rangeHeader, stats.size);
    if (rangeBlind || !range) {
      headers["content-length"] = String(stats.size);
      response.writeHead(200, headers);
      createReadStream(filePath).pipe(response);
      return;
    }
    headers["accept-ranges"] = "bytes";
    headers["content-range"] = `bytes ${range.start}-${range.endInclusive}/${stats.size}`;
    headers["content-length"] = String(range.endInclusive - range.start + 1);
    response.writeHead(206, headers);
    createReadStream(filePath, { start: range.start, end: range.endInclusive }).pipe(response);
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, config.server.host, () => resolve(server));
  });
}

// -- start one server per mode ----------------------------------------------

const servers = [];
const baseUrls = {};
try {
  for (const [index, mode] of config.modes.entries()) {
    const port = config.server.port + index;
    const server = await startServer(mode, port);
    servers.push(server);
    baseUrls[mode.id] = `http://${config.server.host}:${port}`;
    console.log(`[web-matrix] mode ${mode.id} (coop-coep=${mode.coopCoep}) at ${baseUrls[mode.id]}`);
  }
} catch (error) {
  fail(`could not start the harness servers: ${error}`);
}

// -- drive Playwright ---------------------------------------------------------

const outDir = path.join(os.tmpdir(), `lumen-web-matrix-${process.pid}`);
mkdirSync(outDir, { recursive: true });

// The chromium executable the lane will drive: env override (the sandbox
// pins the preinstalled build — downloads are forbidden), else the config's
// relative path under the Playwright browsers root, else @playwright/test's
// own revision-pinned binary (null → unset; CI shape). Whichever ran is
// printed here and recorded in the report's tooling block — the evidence
// always names the browser that actually executed.
const browsersRoot =
  process.env.PLAYWRIGHT_BROWSERS_PATH ??
  path.join(os.homedir(), ".cache", "ms-playwright");
const chromiumExecutable =
  process.env[config.browsers.chromium.executableEnv] ??
  (config.browsers.chromium.executable
    ? path.join(browsersRoot, config.browsers.chromium.executable)
    : null);
if (chromiumExecutable && !statSync(chromiumExecutable).isFile()) {
  fail(`configured chromium executable is not a file: ${chromiumExecutable}`);
}

const cli = path.join(pkgDir, "node_modules", "@playwright", "test", "cli.js");
const playwrightConfig = path.join(pkgDir, "playwright.web.config.ts");
const pwEnv = {
  ...process.env,
  // NEVER download browsers (repo rule): the preinstalled Chromium at
  // the default cache path is the only lane this driver launches.
  PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD: "1",
  WEB_MATRIX_BASE_URLS: JSON.stringify(baseUrls),
  WEB_MATRIX_OUT_DIR: outDir,
  WEB_MATRIX_FIXTURE_BYTES: String(fixtureBytes),
  ...(chromiumExecutable
    ? { [config.browsers.chromium.executableEnv]: chromiumExecutable }
    : {}),
};

console.log(
  `[web-matrix] wasm artifact: ${wasmPath} (${wasmProfile}, ${wasmBytes} bytes, sha256 ${wasmSha.slice(0, 12)}…)`,
);
console.log(
  `[web-matrix] fixture: ${config.server.fixture} (${fixtureBytes} bytes)`,
);
console.log(
  `[web-matrix] chromium executable: ${
    chromiumExecutable ?? "@playwright/test's pinned revision"
  }`,
);
console.log(
  `[web-matrix] chromium launch args: ${
    config.browsers.chromium.launchArgs.length > 0
      ? config.browsers.chromium.launchArgs.join(" ")
      : "(none)"
  }`,
);

// Async on purpose: the harness servers live in THIS process, and a
// synchronous child would starve them — spawnSync blocks the event
// loop, so the kernel would accept Chromium's TCP connections into
// the listen backlog while no HTTP response is ever produced; the
// page then hangs at goto until the test timeout (the failure this
// replaces was diagnosed live: 240 s of net::ERR_ABORTED on goto).
// spawn keeps the driver's servers serving while the child runs.
const child = spawn(process.execPath, [cli, "test", "--config", playwrightConfig], {
  stdio: "inherit",
  cwd: pkgDir,
  env: pwEnv,
});
const playwrightStatus = await new Promise((resolve, reject) => {
  child.once("error", reject);
  child.once("close", (code) => resolve(code ?? 2));
});

for (const server of servers) {
  server.close();
}

// -- merge the fragments into the machine-readable record --------------------

// BCD-cited expectations (doc 09 §8.1, verified 2026-10-19 against
// @mdn/browser-compat-data). These lanes were NOT run in this sandbox
// — no Firefox/WebKit/Safari binaries exist here and downloading is
// forbidden — so each row records the expectation and the next step,
// never a fabricated result (AGENTS §3.4).
function expectationLane(browser, summary, expectations, nextStep) {
  return {
    browser,
    evidence: "not-runnable-in-sandbox",
    summary,
    bcdExpectations: expectations,
    nextStep,
  };
}

const firefoxLane = expectationLane(
  "firefox",
  "Current Firefox desktop (BCD 2026-10-19): WASM MVP+SIMD128 yes; Wasm GC 120+/EH-final 131+; SAB 79+ isolation-gated; Atomics.waitAsync 145+; WebGPU 141+ Windows-only; WebGL2 51+; AudioWorklet 76+; WebCodecs 130+ desktop (Android no).",
  [
    "doc 09 §8.1 row set (Chrome/Edge | Firefox | Safari columns)",
    "v1 suite needs none of SAB/WebGPU: engine core + WebGL2 floor + fetch Range — expected to pass on current Firefox desktop",
    "renderer ladder expectation: WebGL2 active (WebGPU Windows-only per BCD; Firefox Android permanently WebGL2, doc 09 §10)",
  ],
  "Run the same Playwright suite with a real Firefox binary on a dev machine or a CI runner with browsers installed (doc 09 §8.2 scheduled tier [PROPOSAL]); record per-scenario rows into this JSON.",
);

const webkitLane = expectationLane(
  "webkit",
  "Playwright's WebKit ≈ Safari macOS (BCD 2026-10-19): WASM yes; GC 18.2+/EH-final 18.4+; SAB 15.2+ isolation-gated; WebGPU 26+; WebGL2 15+; AudioWorklet 14.1+; OffscreenCanvas 16.4+; rVFC 15.4+.",
  [
    "doc 09 §8.1 row set",
    "v1 suite expected to pass on Safari 18+ desktop via the WebGL2 lane; deltas vs real Safari must be recorded (Playwright WebKit is a build, not the shipping browser)",
  ],
  "Add the weekly Playwright-WebKit smoke tier (doc 09 §8.2 [PROPOSAL]) once a machine with the WebKit build exists; real-Safari validation stays manual per release.",
);

const safariLane = expectationLane(
  "safari",
  "Safari macOS (BCD 2026-10-19): WASM MVP+SIMD128 yes; Wasm GC 18.2+/EH-final 18.4+; threads+SAB 15.2+ (COOP/COEP mandatory); WebGPU 26+; WebGL2 15+; AudioWorklet 14.1+; WebCodecs 16.4+.",
  [
    "doc 09 §8.1 row set",
    "autoplay policy is gesture-gated (doc 09 §5.3): expect gestureRequired on the suspended-context path — the suite's parked/resume scenario exercises exactly that",
  ],
  "Per-release manual protocol on a real Mac (doc 09 §8.2/§11.4): fresh profile → capability probe → fixture corpus → results recorded as feature-matrix evidence.",
);

const safariIosLane = expectationLane(
  "safari-ios",
  "Safari iOS (BCD 2026-10-19): aligns with Safari 18.2+/26+ rows; no SAB without COOP/COEP on iOS; WebGPU iOS 26+; lower memory ceilings (doc 09 §2.4 budget must respect them); fullscreen + autoplay gesture-gated.",
  [
    "doc 09 §8.1 iOS constraints note",
    "physical-device evidence is MANDATORY for any web = verified claim touching SAB, WebGPU or autoplay (doc 09 §11.4)",
  ],
  "Physical iOS device run per release (doc 09 §8.2 manual protocol); no simulator substitute for the memory-pressure + background-throttling rows.",
);

function gitInfo() {
  const run = (args) => {
    const result = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
    return result.status === 0 ? result.stdout.trim() : null;
  };
  return {
    commit: run(["rev-parse", "HEAD"]),
    branch: run(["rev-parse", "--abbrev-ref", "HEAD"]),
  };
}

const pkg = JSON.parse(readFileSync(path.join(pkgDir, "package.json"), "utf8"));
const fragments = config.modes
  .map((mode) => {
    const fragmentPath = path.join(outDir, `chromium-${mode.id}.json`);
    try {
      return JSON.parse(readFileSync(fragmentPath, "utf8"));
    } catch {
      return null;
    }
  })
  .filter((fragment) => fragment !== null);

const chromiumLane = {
  browser: "chromium",
  evidence: "playwright-run",
  playwrightVersion: pkg.devDependencies["@playwright/test"] ?? null,
  modes: fragments,
};

const report = {
  schema: "lumen.web.browser-matrix/1",
  generated: new Date().toISOString(),
  git: gitInfo(),
  tooling: {
    node: process.version,
    harness: "ui/e2e/web (card 7.I1)",
    chromiumExecutable: chromiumExecutable ?? "playwright-pinned",
    chromiumLaunchArgs: config.browsers.chromium.launchArgs,
  },
  artifact: {
    path: path.relative(repoRoot, wasmPath),
    profile: wasmProfile,
    bytes: wasmBytes,
    sha256: wasmSha,
  },
  fixture: {
    path: config.server.fixture,
    bytes: fixtureBytes,
  },
  lanes: [chromiumLane, firefoxLane, webkitLane, safariLane, safariIosLane],
};

const reportDir = resolveRepo(config.report.outputDir);
mkdirSync(reportDir, { recursive: true });
const reportPath = path.join(reportDir, config.report.jsonName);
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

// -- summary -----------------------------------------------------------------

for (const fragment of fragments) {
  const suite = fragment.suite;
  const passed = suite.scenarios.filter((scenario) => scenario.status === "pass").length;
  console.log(
    `[web-matrix] chromium ${fragment.browserVersion} mode=${fragment.mode}: ` +
      `${passed}/${suite.scenarios.length} scenarios passed ` +
      `(crossOriginIsolated=${fragment.crossOriginIsolated})`,
  );
  for (const scenario of suite.scenarios) {
    if (scenario.status === "fail") {
      console.error(`  FAIL ${scenario.name}: ${scenario.error}`);
    }
  }
}
if (fragments.length < config.modes.length) {
  console.error(
    `[web-matrix] WARNING: ${config.modes.length - fragments.length} mode fragment(s) missing — the JSON records only what ran`,
  );
}
console.log(`[web-matrix] record written: ${path.relative(repoRoot, reportPath)}`);
process.exit(playwrightStatus);
