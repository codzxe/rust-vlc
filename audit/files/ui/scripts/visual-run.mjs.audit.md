# AUDIT: ui/scripts/visual-run.mjs
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 63 / 5 top-level blocks (arg parse, path resolution, pwArgs build, spawn, exit propagation) — no functions declared |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (developer-tool chain): `cargo xtask visual` (tools/xtask/src/cmd/visual.rs, registered in the xtask registry) → `npm run visual` (package.json:17) → this script → @playwright/test cli.js → e2e/visual/visual.spec.ts; plus 4 sibling npm scripts (visual:pr/nightly/update/seed) and CI lanes in .github/workflows/ui.yml. Not exercised in the audit baseline (INDEX #21-24 = npm ci/build/vitest/checks only) — execution evidence is the committed MATRIX/README records (cited below). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | package.json scripts read (5 visual entries); playwright.visual.config.ts read in full; e2e/visual/{manifest.ts,visual.spec.ts,README.md,MATRIX-20261113.md} read (route math checked: 25×2×2×2+16=216; PR=50); `ls baselines/linux | wc -l` → 216 (matches manifest math EXACTLY; 1280x720-1x subset = 66 = 50 PR + 16 accent); tools/xtask/src/cmd/visual.rs read (shells out, verbatim exit); rg over .github/workflows/ui.yml (visual + visual-nightly + baseline-seed lanes) |

## 1. One-paragraph verdict
Clean, honest driver — every claim in its header checks out: strict argument parsing with unknown-arg rejection (exit 2), verbatim Playwright exit-code propagation (`child.status ?? 2`, mirrored by the xtask caller), zero new dependencies (only node builtins + spawning @playwright/test@1.64.1's own cli.js, already a devDep), and Windows-portable construction (spawnSync of process.execPath, no shell, no env-var prefix). The downstream harness is complete and internally consistent (216 committed Linux baselines = the manifest's exact 25×2×2×2+16 math), and the script has RECORDED successful executions (MATRIX-20261113.md: nightly 216/216 exit 0; PR 50/50 exit 0; README: seeded 2-px proof fails 8/8 exit 1 — the acceptance evidence). Two hygiene nits only: an undocumented `--update-baselines` alias and no `--help` path.

## 2. What actually works (verified)
- Wiring chain verified end to end: package.json:17-21 (5 scripts), tools/xtask/src/cmd/visual.rs (usage comment matches this script's surface exactly; verbatim exit forwarding), .github/workflows/ui.yml (PR-lite + nightly + blocked baseline-seed lanes).
- Baseline-set integrity: `ls e2e/visual/baselines/linux | wc -l` = **216** = manifest.ts's own arithmetic (25 routes × 2 viewports × 2 DPI × 2 themes + 16 accent cases); the 1280×720-1× subset = 66 = 50 PR cases + 16 accent pins — the committed PNG set is exactly the manifest, no strays, no gaps.
- Recorded executions (not re-run here — sandbox has no playwright/dev-server budget, per instructions): MATRIX-20261113.md:65-70 — "`node scripts/visual-run.mjs --mode=nightly --update` — 216/216 re-generated, 216 passed (8.2m), exit 0", "`npm run visual:pr` — 50 passed (1.6m), exit 0 — zero diffs"; README.md:13-45 — seeded proof: `npm run visual:seed` → "8 failed", `echo $?` → `1` (the failure mode the script exists to prove it propagates).
- Env contract matches the consumer: visual.spec.ts:30-31 reads `VISUAL_MODE === "nightly" ? "nightly" : "pr"` and `SEED_2PX === "1"` — the script always sets both (lines 55), so no undefined-env case exists.
- Path resolution is cwd-independent (`import.meta.url` → scripts/ → ui/) and the missing-node_modules case is handled with a distinct hint (lines 58-62), not a stack trace.

## 3. Findings
### F-001 [S4]
Where: ui/scripts/visual-run.mjs:33
Code:
```js
  } else if (arg === "--update" || arg === "--update-baselines") {
```
What is wrong: The parser accepts `--update-baselines` as an alias, but neither usage string mentions it — not the header comment (line 10: `[--mode=pr|nightly] [--update] [--seed]`) and not the error usage (line 39, identical text).
Why it matters: A hidden alias is an undocumented interface: a user cannot discover it, and a future refactor that "cleans up the unused flag" can break callers who found it only by reading the source. Either it is supported (document it) or it is dead (delete it).
What correct looks like: Add `[--update|--update-baselines]` to both usage strings, or drop the alias.
Confidence: HIGH — Verification: read; both usage strings checked.

### F-002 [S4]
Where: ui/scripts/visual-run.mjs:37-41
Code:
```js
  } else {
    console.error(`visual-run: unknown argument "${arg}"`);
    console.error("usage: node scripts/visual-run.mjs [--mode=pr|nightly] [--update] [--seed]");
    process.exit(2);
  }
```
What is wrong: There is no `--help`/`-h` path — asking for help lands in the unknown-argument branch and exits 2 (hard error), printing the message on stderr framed as a complaint.
Why it matters: Exit 2 is the documented "hard error" code propagated to `cargo xtask visual`; a help request is not an error and pollutes any wrapper that logs non-zero exits (the xtask lane would record a spurious hard error for `--help`).
What correct looks like: `if (arg === "--help" || arg === "-h") { console.log(usage); process.exit(0); }` before the strict parse.
Confidence: HIGH — Verification: read; full parse loop traced (lines 30-42).

## 4. Per-function table (top-level blocks — the script is a straight-line driver)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| shebang + header docs | 1-19 | REAL | — | claims verified: cross-platform (no shell/env-prefix), zero deps, exit semantics |
| arg parse loop | 25-42 | REAL | S4 | strict; last-wins on repeated --mode (harmless); hidden alias (F-001); no help path (F-002) |
| path resolution | 44-46 | REAL | — | import.meta.url-based, cwd-independent; cli.js under node_modules/@playwright/test (devDep 1.64.1, package.json:40) |
| pwArgs build + echo | 48-51 | REAL | — | `--update` → `--update-snapshots` mapping matches Playwright's flag; echo line matches README's recorded output verbatim (`[visual] mode=pr update=false seed=true`) |
| spawn + env | 52-56 | REAL | — | VISUAL_MODE/SEED_2PX always set (consumer defaults can never dangle); stdio inherit; cwd=pkgDir |
| exit propagation | 58-63 | REAL | — | child.error → 2 with install hint; `child.status ?? 2` (signal-kill → 2 = hard error, semantically right) |

## 5. Test honesty (if tests exist in/for this file)
No unit test targets the driver itself (it is 63 lines of process plumbing). Its evidence is the recorded end-to-end runs:
| Record | Would it fail if the script were broken? | Reason |
|---|---|---|
| README.md seeded proof (2026-10-23): visual:seed → "8 failed", exit 1 | YES | the recorded `echo $?` → `1` output IS this script's exit propagation under test — a script that swallowed the child status would have shown 0 |
| MATRIX-20261113.md: nightly --update 216/216 exit 0; PR 50/50 exit 0 | YES (for the spawn/env path) | both modes and the update flag exercised with recorded pass counts matching the manifest math |
| Not covered anywhere: the unknown-arg branch (exit 2), the child.error branch (exit 2 + hint), `--update-baselines` | NOT caught | no record exercises the error paths — reasoned only (F-001/F-002 adjacent) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "both `npm run visual -- --mode=nightly --update` and `cargo xtask visual --mode=nightly --update` work identically" | visual-run.mjs:5-6 | REAL | xtask visual.rs usage comment matches flag-for-flag; both route to this script |
| "zero new npm deps — it spawns @playwright/test's own cli.js" | visual-run.mjs:8 | REAL | imports are node:child_process/url/path only; @playwright/test 1.64.1 pre-existing devDep (package.json:40) |
| "Exit code: Playwright's exit code is propagated verbatim (0 pass, 1 failures, 2 hard error)" | visual-run.mjs:18-19 | REAL | line 63 `process.exit(child.status ?? 2)`; README recorded exit 1 on seeded failure |
| "--seed SEED_2PX=1: inject the 2-px regression proof shift" | visual-run.mjs:16 | REAL | visual.spec.ts:86-93 implements the translateX(2px) injection; README records the failing run |
| (adjacent, workflow doc) "PR-lite lane … 15 routes × 2" | .github/workflows/ui.yml:29 | STALE | manifest.ts (w26.k20) is the 25-route truth; PR lane = 50 cases — the workflow comment predates k19/k20's route growth (drift in ui.yml, not in this script) |

## 7. Dead / duplicated / unreachable code
- `--update-baselines` alias: supportable but undiscoverable (F-001). No dead blocks; every branch reachable by construction.

## 8. Dependencies & flags
Node builtins only (child_process, url, path). Runtime dep on @playwright/test's cli.js (devDep). VISUAL_MODE/SEED_2PX env contract with e2e/visual/visual.spec.ts (verified both sides). No feature flags. The dev server is started by Playwright's webServer (npm run dev --port 5173 --strictPort, 127.0.0.1 per contract §9), not by this script — correct separation.

## 9. Open questions for the next agent
- The stale "15 routes" count in .github/workflows/ui.yml:29 (vs the 25-route manifest) is a one-line doc fix for whoever owns that file; worth a drift-check of the other counts in that header.
- No local run was possible (no playwright/dev-server budget in this sandbox, and instructions forbid npm runs) — the MATRIX/README records stand as the execution evidence; a Wave-16 orchestrator run of `cd ui && npm run visual:pr` would re-confirm on the audit tree (REQUEST-ORCH-RUN candidate, non-blocking: recorded 2026-11-13 evidence exists).
