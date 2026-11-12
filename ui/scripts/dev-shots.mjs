#!/usr/bin/env node
/* global window: readonly -- the addInitScript callbacks below evaluate in the PAGE context; window is the browser global (eslint no-undef). */
// dev-shots.mjs — wave-26 agent screenshot tool (contract §4.7 / §5).
//
// Usage:
//   node dev-shots.mjs <name>:<route> [<name2>:<route2> ...] [--out DIR]
//                      [--port 5173] [--size 1440x900] [--light] [--accent ID]
//                      [--wait 800] [--no-seed]
//
//   <name>:<route>  capture pairs, e.g. player:/player  eq:/player (tools)
//   --out DIR       output dir (default /tmp/shots)
//   --size WxH      viewport (default 1440x900; contract spot-check 1024x768)
//   --light         seed persisted interface.theme=light BEFORE load
//   --accent ID     seed persisted global.accent-color (orange|red|yellow|...)
//   --wait MS       extra settle after networkidle (default 800)
//   --no-seed       do NOT seed the onboarding-complete gate
//
// Behavior: seeds `lumen.onboarding.v1` (complete) for every route except
// /onboarding itself — same shape the e2e a11y suite seeds — so gated routes
// render. Waits for networkidle + settle, captures the viewport to
// <out>/<name>.png and logs console errors to stdout (quality bar: zero).
import { createRequire } from "node:module";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

// Resolve playwright from the ui workspace regardless of where this copy
// lives (agents run it from /home/z/my-project/scripts/, the durable copy
// lives in ui/scripts/).
let chromium;
for (const base of [
  resolve(import.meta.dirname, ".."), // inside ui/scripts/
  resolve(import.meta.dirname, "../rust-vlc/ui"), // sandbox scripts dir
]) {
  try {
    const req = createRequire(resolve(base, "package.json"));
    ({ chromium } = req("playwright"));
    break;
  } catch {
    /* try next */
  }
}
if (!chromium) {
  console.error("dev-shots: cannot resolve playwright from ui/node_modules");
  process.exit(2);
}

const args = process.argv.slice(2);
const pairs = [];
const opt = { out: "/tmp/shots", port: 5173, size: "1440x900", light: false, accent: null, wait: 800, seed: true };
for (let i = 0; i < args.length; i++) {
  const a = args[i];
  if (a.startsWith("--")) {
    if (a === "--out") opt.out = args[++i];
    else if (a === "--port") opt.port = Number(args[++i]);
    else if (a === "--size") opt.size = args[++i];
    else if (a === "--light") opt.light = true;
    else if (a === "--accent") opt.accent = args[++i];
    else if (a === "--wait") opt.wait = Number(args[++i]);
    else if (a === "--no-seed") opt.seed = false;
    else { console.error(`dev-shots: unknown option ${a}`); process.exit(2); }
  } else if (a.includes(":")) {
    const idx = a.indexOf(":");
    pairs.push({ name: a.slice(0, idx), route: a.slice(idx + 1) });
  } else {
    console.error(`dev-shots: bad pair ${a} (want <name>:<route>)`); process.exit(2);
  }
}
if (pairs.length === 0) {
  console.error("dev-shots: no <name>:<route> pairs given");
  process.exit(2);
}
const [w, h] = opt.size.split("x").map(Number);
mkdirSync(opt.out, { recursive: true });

const browser = await chromium.launch();
let failures = 0;
for (const { name, route } of pairs) {
  const page = await browser.newPage({ viewport: { width: w, height: h } });
  const consoleErrors = [];
  page.on("console", (msg) => { if (msg.type() === "error") consoleErrors.push(msg.text()); });
  page.on("pageerror", (err) => consoleErrors.push(`pageerror: ${err.message}`));
  await page.addInitScript(({ route: r, seed, light, accent }) => {
    if (seed && !r.startsWith("/onboarding")) {
      window.localStorage.setItem("lumen.onboarding.v1", JSON.stringify({
        complete: true,
        choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
      }));
    }
    const overrides = {};
    if (light) overrides["interface.theme"] = "light";
    if (accent) overrides["global.accent-color"] = accent;
    if (Object.keys(overrides).length > 0) {
      window.localStorage.setItem("lumen.settings.overrides.v1", JSON.stringify(overrides));
    }
  }, { route, seed: opt.seed, light: opt.light, accent: opt.accent });
  try {
    await page.goto(`http://localhost:${opt.port}${route}`, { waitUntil: "networkidle", timeout: 20000 });
    await page.waitForTimeout(opt.wait);
    const file = resolve(opt.out, `${name}.png`);
    await page.screenshot({ path: file });
    if (consoleErrors.length > 0) {
      failures++;
      writeFileSync(resolve(opt.out, `console-${name}.log`), consoleErrors.join("\n") + "\n");
      console.log(`✗ ${name} (${route}) → ${file} — ${consoleErrors.length} CONSOLE ERROR(S), see console-${name}.log`);
    } else {
      console.log(`✓ ${name} (${route}) → ${file}`);
    }
  } catch (err) {
    failures++;
    console.log(`✗ ${name} (${route}) — FAILED: ${err.message.split("\n")[0]}`);
  }
  await page.close();
}
await browser.close();
process.exit(failures > 0 ? 1 : 0);
