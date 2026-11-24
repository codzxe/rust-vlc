# AUDIT: ui/playwright.config.ts
| Field | Value |
|---|---|
| Auditor | W16-A03 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 56 / 1 defineConfig |
| File verdict | REAL (real e2e smoke config; honestly excludes the visual/web lanes from the default command) |
| Reachable from shipped entry point? | N/A — test config; consumer = `npm run test:e2e` + ui.yml CI |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read full file; rg consumers (package.json, .github/workflows/ui.yml); baseline rc-smoke.txt |

## 1. Verdict
The plain e2e-smoke Playwright config: testDir ./e2e, chromium headless, baseURL
127.0.0.1:5173 against a webServer `npm run dev --port 5173 --strictPort`, 240s server
budget, explicit reporters. Its two load-bearing decisions are honest and documented:
testIgnore keeps visual/ and web/ out of the plain command (each harness has an owner config
— a documented trap fix, :8-16), and reuseExistingServer is disabled under CI but allowed
locally (:43-47).

## 2. What actually works (verified)
- rc-smoke baseline: `web-ui-build` and `browser-load` lanes PASS (audit/runs/baseline/
  rc-smoke.txt) — the dev-server + browser path this config owns is proven in the
  orchestrator baseline (exact lane names cited; UNVERIFIED whether the browser-load lane
  uses THIS config vs the visual config, but the dev-server + chromium load shape matches).
- Trap fix verified: `testIgnore: ["**/visual/**", "**/web/**"]` (:21) — plain
  `npx playwright test` cannot write stray snapshots (the comment documents the historical
  failure mode).
- webServer URL contract matches the app's documented 127.0.0.1 bind (:34-38 comment cites
  the contract; command spins the same port with --strictPort).

## 3. Findings
F-001 [S4]
Where: ui/playwright.config.ts:47
Code:
    reuseExistingServer: !process.env.CI,
What is wrong: locally, a STALE dev server on 5173 (different branch/worktree, different
API) is silently reused — the config itself documents the danger (:43-47) and chooses the
documented Playwright pattern, but any local staleness produces misleading e2e results with
no warning. CI is the honest path; local is convenience-over-correctness.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: ui/playwright.config.ts:41
Code:
    command: "npm run dev -- --port 5173 --strictPort",
What is wrong: the e2e smoke runs against the DEV server (unminified, HMR runtime) — the
shipped build's e2e behavior is never exercised by this lane (only the separate web
browser-load lane touches a built artifact). Performance/behavior drift between dev and
release builds stays undetected here.
Confidence: HIGH — Verification: read + baseline lane names.

## 4. Per-key table (config inventory)
| Key | Verdict | Notes |
|---|---|---|
| testDir ./e2e | REAL | smoke specs live there (rg'd earlier) |
| testIgnore visual/web | REAL | trap fix, documented |
| timeout 30s / fullyParallel | REAL | defaults sane |
| reporters list+html | REAL | html never auto-opens; artifact upload wired in ui.yml |
| chromium headless | REAL | matches visual lanes |
| baseURL 127.0.0.1:5173 | REAL | contract-cited |
| webServer dev --strictPort | REAL | F-002 (dev server, not build) |
| reuseExistingServer !CI | REAL | F-001 (local staleness) |
| timeout 240s server | REAL | justified by cited CI run 43370495290 cold-start |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Browsers ARE installed now (chromium 156.0.8078.4…)" | :3-5 | UNVERIFIED | no local playwright run possible in this recovery env; baseline browser-load PASS is indirect |
| snapshotDir trap rationale | :8-16 | REAL | the exclude exists and matches the harness ownership docs |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
@playwright/test; declares a local `process` type (zero-dep node-env typing, :26-28) —
documented repo-rule workaround, not a hack hiding anything.

## 8. Open questions for the next agent
None blocking.
