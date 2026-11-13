#!/usr/bin/env node
// lint-css — stylelint launcher that prefers the LOCAL install (w26.k20,
// j20 P3-17).
//
// `npm run lint:css` previously invoked
//   npx -p stylelint@18.0.0 -p stylelint-config-standard@41.0.0 stylelint …
// because the wave sandbox's ui/node_modules was a shared symlink without
// stylelint. That path hits the NETWORK on a cold npx cache — verboten in
// no-install sandboxes (contract §4.1). ui/node_modules now carries the
// real install (stylelint 18.0.0 + config-standard 41.0.0 — the exact
// versions the npx form pinned, from package.json devDependencies), so:
//
//   1. ui/node_modules/.bin/stylelint exists → run it (offline, canonical);
//   2. otherwise → fall back to the historical npx form with a loud notice
//      (real installs and CI always take path 1; path 2 exists so the gate
//      keeps working in a degraded checkout instead of silently passing).
//
// Argument surface is unchanged: everything after this script's argv[2:]
// is forwarded to stylelint verbatim (`npm run lint:css` passes the glob).

import { spawnSync } from "node:child_process";
import { accessSync, constants } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const uiRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const localBin = resolve(uiRoot, "node_modules", ".bin", "stylelint");
const args = process.argv.slice(2);

const canRun = (path) => {
  try {
    accessSync(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
};

let command;
let argv;
if (canRun(localBin)) {
  command = localBin;
  argv = args;
} else {
  console.error(
    "[lint-css] no local stylelint in ui/node_modules — falling back to `npx -p` " +
      "(may touch the network on a cold cache; run `npm install` in ui/ to restore the offline path).",
  );
  command = "npx";
  argv = [
    "-p",
    "stylelint@18.0.0",
    "-p",
    "stylelint-config-standard@41.0.0",
    "stylelint",
    ...args,
  ];
}

const child = spawnSync(command, argv, { stdio: "inherit", cwd: uiRoot });
if (child.error) {
  console.error(`[lint-css] failed to launch stylelint: ${child.error.message}`);
  process.exit(2);
}
process.exit(child.status ?? 2);
