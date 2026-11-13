#!/usr/bin/env node
// dev-shots-drift-check — the dual-copy drift guard for dev-shots.mjs
// (w26.k20, j20 P2-6).
//
// Two durable copies of the wave-26 agent screenshot tool exist BY DESIGN:
//   * ui/scripts/dev-shots.mjs        — the repo-committed CANONICAL copy
//   * <sandbox>/scripts/dev-shots.mjs — the agent-facing copy CONTRACT §4.7
//     points 260 agents at (outside the repo; /home/z/my-project/scripts/
//     in the wave sandbox)
// They were byte-identical but nothing verified that — an edit to either
// would silently fork the tool every agent depends on. This check fails
// loudly the moment the two diverge (content hash comparison; the repo copy
// is the source of truth — refresh the outer copy from it).
//
// Resolution of the outer copy: DEV_SHOTS_OUTER_COPY env override → the
// sandbox sibling layout (<repo>/../scripts/dev-shots.mjs). When no outer
// copy exists (CI, fresh clones), the check PASSES with a notice — the
// repo copy is self-sufficient; only the sandbox runs both.
//
// Wired as `npm run dev-shots:check` (the tokens:check/settings:check/
// hotkeys:check/l10n:check family).

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const uiRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoCopy = resolve(uiRoot, "scripts/dev-shots.mjs");

const outerCandidates = [
  process.env.DEV_SHOTS_OUTER_COPY,
  resolve(uiRoot, "../../scripts/dev-shots.mjs"),
].filter((p) => typeof p === "string" && p.length > 0);

const sha256 = (path) =>
  createHash("sha256").update(readFileSync(path)).digest("hex");

const repoHash = sha256(repoCopy);

let outer = null;
for (const candidate of outerCandidates) {
  try {
    readFileSync(candidate); // exists?
    outer = candidate;
    break;
  } catch {
    /* try the next candidate */
  }
}

if (outer === null) {
  console.log(
    `OK dev-shots:check — repo copy canonical (sha256 ${repoHash.slice(0, 12)}…); ` +
      "no agent-facing outer copy in this checkout (CI/fresh clone) — nothing to drift.",
  );
  process.exit(0);
}

const outerHash = sha256(outer);
if (repoHash === outerHash) {
  console.log(
    `OK dev-shots:check — repo copy and agent-facing copy byte-identical (sha256 ${repoHash.slice(0, 12)}…).`,
  );
  process.exit(0);
}

console.error(
  "FAIL dev-shots:check — the two dev-shots.mjs copies have DIVERGED (j20 P2-6):\n" +
    `  repo copy (canonical): ${repoCopy}  sha256 ${repoHash.slice(0, 12)}…\n` +
    `  agent-facing copy:     ${outer}  sha256 ${outerHash.slice(0, 12)}…\n` +
    "The repo-committed copy is the source of truth. Refresh the outer copy:\n" +
    `  cp ${repoCopy} ${outer}\n` +
    "then re-run `npm run dev-shots:check`.",
);
process.exit(1);
