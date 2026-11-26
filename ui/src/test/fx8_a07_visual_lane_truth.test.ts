// FX8-A07 (FIX-PLAN §4 F2.2 tail) — the visual lane's honesty guard.
//
// The audit finding: the full screenshot-diff matrix was schedule-only (the
// old `visual-nightly` job), so a visual regression could ship undetected
// for up to 24h, and the workflow's lane labels carried stale counts
// (30/136 vs the live 50/216 — audit files/.github/workflows/ui.yml.audit.md
// F-001). The fix un-gated the full matrix (renamed `visual-full`) so it
// gates every push/PR, kept the PR-lite lane as the fast signal, and left
// ONLY the macOS/Windows baseline-SEED job schedule-gated (it CAPTURES —
// not compares — on OSes whose baselines cannot be produced from this
// repo's Linux sandbox; a compare there fails by design until the captured
// sets are committed, which is honest, not fake green).
//
// This guard pins that state so drift is loud (the ui job's vitest lane runs
// per-PR, before any Playwright case):
//   1. e2e/visual/manifest.ts is evaluated by node ITSELF (a type-stripping
//      spawn — vitest transforms only src/**, so spawning the real file
//      imports exactly what the visual lane runs, not a re-typed copy) and
//      the case counts are pinned: 50 PR-lite / 216 full (grid 200 + 16
//      accent samples) over 25 routes;
//   2. .github/workflows/ui.yml must run visual-full on every push/PR (no
//      event gate on it), keep the baseline-seed gate by design, and label
//      the lanes with the counts the live manifest evaluates to — manifest
//      growth without a label bump fails here (the stale-30/136 class).

import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { describe, expect, it } from "vitest";

// import.meta.url slicing keeps the real filesystem path (the generated.test.ts
// precedent — Vite rewrites `new URL(literal, import.meta.url)` into an asset
// import, string slicing does not).
const dirUrl = import.meta.url.slice(0, import.meta.url.lastIndexOf("/") + 1);
const toPath = (url: string): string => url.replace(/^file:\/\//, "");
// ui/src/test/ → ui/e2e/visual/manifest.ts (2 ups)
const manifestPath = toPath(`${dirUrl}../../e2e/visual/manifest.ts`);
// ui/src/test/ → repo root (3 ups) → the owning workflow
const uiYml = readFileSync(toPath(`${dirUrl}../../../.github/workflows/ui.yml`), "utf8");

/** GitHub Actions job block keyed by job id — 2-space-indent keys, the block
 * spans until the next job key or EOF. Enough structure for gating
 * assertions without a yaml dependency (the ui package has none). */
function jobBlock(id: string): string {
  const lines = uiYml.split("\n");
  const start = lines.findIndex((l) => l === `  ${id}:`);
  expect(start, `job "${id}" must exist in .github/workflows/ui.yml`).toBeGreaterThanOrEqual(0);
  let end = lines.length;
  for (let i = start + 1; i < lines.length; i += 1) {
    if (/^  [a-z][a-z0-9-]*:$/.test(lines[i] ?? "")) {
      end = i;
      break;
    }
  }
  return lines.slice(start, end).join("\n");
}

/** Evaluated manifest truth — the same module the visual harness imports
 * (visual.spec.ts `import { MANIFEST, caseId } from "./manifest"`), loaded
 * by node's own type stripping (default on since node 23.6; the repo pins
 * node 24 in CI and local dev). */
interface ManifestCounts {
  total: number;
  grid: number;
  samples: number;
  pr: number;
  routes: number;
  defaultAccent: string;
}

const PROBE = `
const m = await import(process.argv[1]);
const grid = m.MANIFEST.filter((c) => c.accent === m.DEFAULT_ACCENT);
const samples = m.MANIFEST.filter((c) => c.accent !== m.DEFAULT_ACCENT);
console.log(JSON.stringify({
  total: m.MANIFEST.length,
  grid: grid.length,
  samples: samples.length,
  pr: m.MANIFEST.filter((c) => c.pr).length,
  routes: m.ROUTES.length,
  defaultAccent: m.DEFAULT_ACCENT,
}));`;

function evalManifest(): ManifestCounts {
  // The process global is reached via the generated.test.ts seam (the ui
  // tsconfig auto-includes only vite/client types).
  const execPath = (globalThis as unknown as { process: { execPath: string } }).process.execPath;
  const child = spawnSync(execPath, ["--input-type=module", "--eval", PROBE, "--", manifestPath], {
    encoding: "utf8",
  });
  expect(child.error?.message ?? "", "spawning node to evaluate the visual manifest failed").toBe("");
  expect(child.status, `the manifest probe exited non-zero: ${child.stderr}`).toBe(0);
  return JSON.parse(child.stdout) as ManifestCounts;
}

/** A lane restricted to schedule/dispatch events (the re-gating this guard
 * exists to catch); `if: always()` keep-going upload steps are NOT gates. */
const EVENT_GATE = /if:\s*github\.event_name/;

describe("fx8_a07: visual lane truth (F2.2 — full matrix per-PR, honest counts)", () => {
  it("the manifest evaluates via node to the pinned sizes: 50 PR-lite / 216 full (grid 200 + 16 samples) over 25 routes", () => {
    expect(evalManifest()).toEqual({
      total: 216,
      grid: 200,
      samples: 16,
      pr: 50,
      routes: 25,
      defaultAccent: "orange",
    });
  });

  it("ui.yml runs the full 216-case matrix on every push/PR (visual-full carries no event gate)", () => {
    const full = jobBlock("visual-full");
    expect(full).toContain("--mode=nightly"); // the full-matrix CLI surface
    expect(
      full,
      "visual-full must not be event-gated — the audit finding was a visual regression shipping undetected for up to 24h (FIX-PLAN §4 F2.2)",
    ).not.toMatch(EVENT_GATE);
  });

  it("ui.yml keeps the PR-lite 50-case lane per-PR as the fast signal (visual, --mode=pr, un-gated)", () => {
    const prLane = jobBlock("visual");
    expect(prLane).toContain("--mode=pr");
    expect(prLane).not.toMatch(EVENT_GATE);
  });

  it("only the baseline-SEED job stays schedule-gated (macOS/Windows capture, by design)", () => {
    const seed = jobBlock("visual-baseline-seed");
    expect(seed).toContain("--mode=nightly --update"); // capture, not compare
    expect(seed).toMatch(
      /if:\s*github\.event_name == 'schedule' \|\| github\.event_name == 'workflow_dispatch'/,
    );
    // No other job may carry an event gate — that is the re-gating drift.
    for (const id of ["ui", "visual", "visual-full"]) {
      expect(jobBlock(id), `job "${id}" must gate every push/PR`).not.toMatch(EVENT_GATE);
    }
  });

  it("the workflow's lane labels agree with the live manifest (no stale 30/136 counts)", () => {
    const counts = evalManifest();
    const prLabel = jobBlock("visual").match(/PR-lite lane — (\d+) cases/);
    const fullLabel = jobBlock("visual-full").match(/full matrix — (\d+) cases/);
    expect(prLabel?.[1], "the visual job's name must carry the live PR-lite case count").toBe(
      String(counts.pr),
    );
    expect(fullLabel?.[1], "the visual-full job's name must carry the live full-matrix case count").toBe(
      String(counts.total),
    );
    const headerCounts = uiYml.match(/(\d+) routes × 2\s*\n?#?\s*modes = (\d+) cases/);
    expect(headerCounts?.[1], "the header's route count must match ROUTES.length").toBe(
      String(counts.routes),
    );
    expect(headerCounts?.[2], "the header's PR case count must match the manifest").toBe(
      String(counts.pr),
    );
    // The stale labels the audit flagged (ui.yml.audit.md F-001) stay dead.
    expect(uiYml).not.toMatch(/\b30 cases\b/);
    expect(uiYml).not.toMatch(/\b136 cases\b/);
  });
});
