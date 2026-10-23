#!/usr/bin/env node
// Cross-platform driver for the visual harness (card 4.G1).
//
// Maps the task-card CLI surface onto Playwright flags and env, so both
// `npm run visual -- --mode=nightly --update` and `cargo xtask visual
// --mode=nightly --update` work identically on any OS (env-var prefixes
// like `VISUAL_MODE=pr …` are not portable to Windows npm scripts, hence
// this shim; zero new npm deps — it spawns @playwright/test's own cli.js).
//
// Usage: node scripts/visual-run.mjs [--mode=pr|nightly] [--update] [--seed]
//
//   --mode=pr       PR-lite lane (default): 1280×720 @1×, dark + light
//   --mode=nightly  full matrix: viewports × DPI × modes
//   --update        re-record baselines (--update-snapshots); pair with
//                   --mode=nightly to re-pin the full matrix (4.G2)
//   --seed          SEED_2PX=1: inject the 2-px regression proof shift
//
// Exit code: Playwright's exit code is propagated verbatim (0 pass,
// 1 failures, 2 hard error) so `cargo xtask visual` stays honest.

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const args = process.argv.slice(2);
let mode = "pr";
let update = false;
let seed = false;

for (const arg of args) {
  if (arg === "--mode=pr" || arg === "--mode=nightly") {
    mode = arg.slice("--mode=".length);
  } else if (arg === "--update" || arg === "--update-baselines") {
    update = true;
  } else if (arg === "--seed") {
    seed = true;
  } else {
    console.error(`visual-run: unknown argument "${arg}"`);
    console.error("usage: node scripts/visual-run.mjs [--mode=pr|nightly] [--update] [--seed]");
    process.exit(2);
  }
}

const pkgDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cli = path.join(pkgDir, "node_modules", "@playwright", "test", "cli.js");
const config = path.join(pkgDir, "playwright.visual.config.ts");

const pwArgs = ["test", "--config", config];
if (update) pwArgs.push("--update-snapshots");

console.log(`[visual] mode=${mode} update=${update} seed=${seed}`);
const child = spawnSync(process.execPath, [cli, ...pwArgs], {
  stdio: "inherit",
  cwd: pkgDir,
  env: { ...process.env, VISUAL_MODE: mode, SEED_2PX: seed ? "1" : "0" },
});

if (child.error) {
  console.error(`visual-run: failed to launch Playwright: ${child.error.message}`);
  console.error("hint: run `npm install` in ui/ once (npx playwright install chromium if fresh clone)");
  process.exit(2);
}
process.exit(child.status ?? 2);
