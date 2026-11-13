import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { get as httpGet } from "node:http";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

// l10n-pseudo-probe — the ?locale=pseudo clipping/overflow probe (card 4.H3,
// doc 07 §6.3). Boots its own throwaway dev server on :5199, walks the full
// routable inventory under the deterministic pseudo locale (~30% longer
// strings) and reports per-route: horizontal page overflow, ellipsized+
// actually-clipped text nodes, and leftover pseudo brackets.
//
// w26.k20 (j20 P2-8) hardening — the pre-k20 probe was fragile in four ways,
// all fixed here:
//   1. fixed 5 s sleep for server readiness → poll the port (250 ms
//      interval, 60 s cap, fail fast if the server process dies first);
//   2. no try/finally around the browser walk → any page.goto failure
//      leaked a detached vite server on :5199; the server is now stopped in
//      a finally block no matter how the walk ends;
//   3. terminal `process.kill(-pid)` threw if npm had already died → the
//      kill is guarded (process group first, direct pid fallback, exitCode
//      checked);
//   4. the route list was 5 routes AND pre-dated w26.i20's app-wide
//      first-run gate — without the onboarding-complete seed every gated
//      route was silently screenshotting the WIZARD. The probe now seeds
//      lumen.onboarding.v1 (same shape as dev-shots / e2e a11y / the visual
//      harness) and walks the full 24-route inventory (the /dev/tokens
//      fixture is skipped: it renders raw token swatches, not copy).
//   5. (found live by the relaunch: the probe's own throwaway server never
//      answered) vite's DEFAULT `localhost` binding is ::1-only in this
//      sandbox, so 127.0.0.1:5199 was connection-refused forever — the
//      spawn now passes `--host 127.0.0.1` explicitly (contract §9, same
//      discipline as the Lead's 5173 server; verified: 200 on IPv4).
//
// 127.0.0.1, not localhost — contract §9 (the dev server binds 127.0.0.1).
//
// Exit code: 0 with the JSON report on a completed walk (clipping findings
// are REPORT data, not failures — triage is a human call); 1 if the server
// never became ready or any route failed to load (probe infrastructure
// failure, not a finding).

const UI = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const PROBE_DIR = resolve(UI, "pseudo-probe");
mkdirSync(PROBE_DIR, { recursive: true });

const PORT = 5199;
const BASE_URL = `http://127.0.0.1:${PORT}`;

/** The full routable inventory (w26.k20): 12 non-settings renderable routes
 * (dev fixture excluded) + all 13 settings areas — the same truth the
 * visual manifest and the a11y sweep walk. */
const routes = [
  "/player",
  "/player/music",
  "/miniplayer",
  "/playlist",
  "/media-info",
  "/discovery",
  "/convert",
  "/settings/global",
  "/settings/interface",
  "/settings/privacy",
  "/settings/playback",
  "/settings/video",
  "/settings/audio",
  "/settings/subtitles",
  "/settings/controls",
  "/settings/library",
  "/settings/network",
  "/settings/performance",
  "/settings/recording",
  "/settings/extensions",
  "/onboarding",
  "/keyboard-shortcuts",
  "/open-network-stream",
  "/no-such-route", // the catch-all 404
];

/** One HEAD-ish GET against the dev server; resolves false on any error. */
function probeUrl(url) {
  return new Promise((resolveProbe) => {
    const req = httpGet(url, (res) => {
      res.resume();
      resolveProbe(res.statusCode !== undefined && res.statusCode > 0 && res.statusCode < 500);
    });
    req.on("error", () => resolveProbe(false));
    req.setTimeout(2000, () => {
      req.destroy();
      resolveProbe(false);
    });
  });
}

/** Poll the dev server until it answers; fail fast if the process dies. */
async function waitForServer(server, timeoutMs = 60_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (server.exitCode !== null || server.signalCode !== null) {
      throw new Error(
        `dev server exited before answering ${BASE_URL} (code ${server.exitCode ?? ""}${server.signalCode ?? ""})`,
      );
    }
    if (await probeUrl(BASE_URL)) {
      return;
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`dev server not answering ${BASE_URL} within ${timeoutMs} ms`);
}

/** Stop the detached server group; never throws (ESRCH = already gone). */
function stopServer(server) {
  if (server.exitCode !== null || server.signalCode !== null) {
    return;
  }
  try {
    process.kill(-server.pid, "SIGTERM"); // whole detached group (npm + vite)
  } catch {
    try {
      server.kill("SIGTERM");
    } catch {
      /* already dead — nothing to clean up */
    }
  }
}

const server = spawn(
  "npm",
  ["run", "dev", "--", "--port", String(PORT), "--strictPort", "--host", "127.0.0.1"],
  { cwd: UI, stdio: "ignore", detached: true },
);

let browser = null;
let loadFailures = 0;
try {
  await waitForServer(server);
  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });

  // First-run gate (w26.i20): without the seed, every gated route would
  // render the onboarding wizard instead of its own screen. Registered ONCE
  // — addInitScript callbacks run on every subsequent navigation, and
  // /onboarding itself stays unseeded so the wizard renders its real
  // first-run state. Same store shape as dev-shots / e2e a11y / visual.
  await page.addInitScript(() => {
    if (!window.location.pathname.startsWith("/onboarding")) {
      window.localStorage.setItem(
        "lumen.onboarding.v1",
        JSON.stringify({
          complete: true,
          choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
        }),
      );
    }
  });

  const report = [];
  for (const route of routes) {
    try {
      await page.goto(`${BASE_URL}${route}?locale=pseudo`, {
        waitUntil: "networkidle",
        timeout: 30_000,
      });
    } catch (err) {
      loadFailures++;
      report.push({ route, error: String(err.message).split("\n")[0] });
      continue;
    }
    await page.waitForTimeout(700);
    const findings = await page.evaluate(() => {
      const doc = document.scrollingElement;
      const pageOverflowX = doc.scrollWidth - window.innerWidth;
      const clipped = [];
      for (const el of document.querySelectorAll("main *, [role] *")) {
        if (el.children.length === 0 && el.textContent && el.textContent.trim().length > 0) {
          const ellipsized = getComputedStyle(el).textOverflow === "ellipsis";
          if (ellipsized && el.scrollWidth > el.clientWidth + 1)
            clipped.push({ tag: el.tagName, cls: String(el.className).slice(0, 60), text: el.textContent.trim().slice(0, 30) });
        }
      }
      return { pageOverflowX, ellipsizedCount: clipped.length, samples: clipped.slice(0, 4), bracketed: document.body.textContent.includes("[") };
    });
    await page.screenshot({ path: resolve(PROBE_DIR, `pseudo-${route.replaceAll("/", "-") || "root"}.png`) });
    report.push({ route, ...findings });
  }
  console.log(JSON.stringify(report, null, 2));
  if (loadFailures > 0) {
    console.error(`l10n-pseudo-probe: ${loadFailures} of ${routes.length} route(s) failed to load`);
    process.exitCode = 1;
  }
} finally {
  if (browser !== null) {
    await browser.close().catch(() => {});
  }
  stopServer(server);
}
