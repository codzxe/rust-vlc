# AUDIT: ui/e2e/visual/visual.spec.ts
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 105 / 1 test factory (manifest-driven: 50 PR cases / 216 nightly cases) + helpers inline |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as a test lane — driven via ui/package.json:17-21 (`visual`, `visual:pr`, `visual:nightly`, `visual:update`, `visual:seed` -> scripts/visual-run.mjs) under ui/playwright.visual.config.ts (testDir ./e2e/visual, snapshotPathTemplate baselines/{platform}); the default playwright.config.ts:20 excludes **/visual/** so the plain e2e run can never trip the snapshot-dir trap |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; rg config/scripts/MATRIX evidence; python manifest cross-check (in PNG audits) |

## 1. One-paragraph verdict
A tight, honest screenshot-diff harness: one test factory walks the manifest cases with per-case viewport/DPI, forces deterministic rendering (reducedMotion, colorScheme pinned, onboarding seeded, data-theme/data-accent applied post-mount, fonts.ready + 250ms settle), clips to the `#root` bounding box so viewport chrome never enters the diff, and compares against pinned per-platform baselines. Tolerances live where Playwright intends them (playwright.visual.config.ts `expect.toHaveScreenshot`: threshold 0.1 — tighter than the 0.2 default — maxDiffPixelRatio 0.001, animations disabled, caret hidden, scale "css"), matching the spec's comment exactly. The suite has a built-in mutation proof (SEED_2PX=1 shifts the shell 2px via transform and MUST fail — wired as `npm run visual:seed`), and committed execution evidence exists: MATRIX-20261113.md:66-69 records 216/216 nightly capture green and PR compare `50 passed`.

## 2. What actually works (verified)
— ui/e2e/visual/MATRIX-20261113.md:66-69: nightly `216/216 re-generated, 216 passed (8.2m), exit 0`; PR capture `50 passed (1.7m)`; PR compare `npm run visual:pr — 50 passed` (line 69).
— Structure: cases filtered by MODE (`MANIFEST.filter((c) => MODE === "nightly" || c.pr)`, :32) = 50 PR / 216 nightly; determinism stack :50-77; clip #root :81-84 (throws if no bounding box — fails loud, not silently empty); SEED_2PX injection :86-94 with transform-not-margin rationale.
— Config wiring verified: tolerances configured at ui/playwright.visual.config.ts `expect.toHaveScreenshot` (threshold 0.1, maxDiffPixelRatio 0.001, animations "disabled", caret "hide", scale "css"); spec comment :96-101 matches config values one-for-one.

## 3. Findings
None. Zero findings — what was checked and how: (a) comment-vs-config tolerance claims cross-checked line-by-line (visual.spec.ts:96-101 vs playwright.visual.config.ts expect block — equal, no comment/code drift); (b) snapshot-dir trap closure verified (playwright.config.ts:20 testIgnore `**/visual/**` — the plain e2e lane cannot run this spec against default snapshot locations); (c) SEED_2PX mutation-proof wired to `npm run visual:seed` (ui/package.json:21) with recorded real-number evidence (docs/waves/w26/reviews/w26.j20.md:91 "The seeded-regression proof (SEED_2PX) is recorded with real numbers"); (d) no skip/only/fixme; (e) clip failure path throws with context (:82-84).

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| MODE/SEED_2PX env reads | 30-31 | REAL | pr default; nightly opt-in |
| cases filter | 32 | REAL | PR lane = manifest `pr` flags |
| test factory | 34-104 | REAL | per-case test.use viewport/DPI (:39-42) |
| emulateMedia + onboarding seed | 50-65 | REAL | deterministic app state, wizard route excluded |
| theme/accent forcing | 68-75 | REAL | data-theme/data-accent + fonts.ready |
| settle wait | 77 | REAL | 250ms token-cascade repaint |
| #root boundingBox clip | 81-84 | REAL | throws when unclippable |
| SEED_2PX style injection | 86-94 | REAL | transform shift; clip rect unaffected by design |
| toHaveScreenshot | 102 | REAL | `{ clip: box }`; tolerances from config (verified equal to comment) |

## 5. Test honesty
| Property | Would it fail if code were broken? | Reason |
|---|---|---|
| baseline pixel drift | YES | pixel comparison at threshold 0.1 / 0.001 ratio — the W25 reskin re-pin (MATRIX-20261113) proves real drift changes fail/force re-pin |
| seeded 2px regression | YES (by construction) | SEED_2PX injects a 2px transform — ~0.3% differing pixels > 0.001 cap → FAIL; recorded with real numbers (w26.j20.md:91) |
| route renders nothing / #root missing | YES | boundingBox null throws |
| wrong theme/accent applied | YES | forced post-mount attributes + baseline pixels |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Tolerance: threshold 0.1 ... + maxDiffPixelRatio 0.001" | visual.spec.ts:96-101 | REAL | playwright.visual.config.ts expect block carries exactly these values |
| "retries=1 ... a real regression fails twice" | visual.spec.ts:20-21 | REAL | playwright.visual.config.ts `retries: 1` with same rationale comment |
| "SEED_2PX=1 ... the harness must FAIL" | visual.spec.ts:14-15 | REAL | `npm run visual:seed` (ui/package.json:21) + recorded numbers (w26.j20.md:91); exact failure output not in my sandbox (no node_modules) — UNVERIFIED locally, verified by committed review record |

## 7. Dead / duplicated / unreachable code
None. `declare const process` (:28) is the documented zero-dep node-env typing seam for e2e tsconfig — commented, not dead.

## 8. Dependencies & flags
@playwright/test only. VISUAL_MODE/SEED_2PX env knobs via scripts/visual-run.mjs:55 (`env: { ...process.env, VISUAL_MODE: mode, SEED_2PX: seed ? "1" : "0" }`).

## 9. Open questions for the next agent
- The MATRIX-20261113 nightly capture is a re-PIN (`--update`) — it proves capture, and the compare lane proves 50 PR cases; nightly 216-case COMPARE green runs are the missing committed artifact (only capture/update recorded).
