# AUDIT: ui/playwright.visual.config.ts
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A03) |
| Date | 2026-06-14 |
| Lines / functions | 91 / 1 default defineConfig export |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — e2e harness config; consumed by `npx playwright test` via scripts/visual-run.mjs:46 (`config = playwright.visual.config.ts`, :55 sets VISUAL_MODE), wired into CI at .github/workflows/ui.yml:148-173 (PR-lite lane) and the 03:41-UTC nightly (:58) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (cross-ref) |
| Commands run | `rg -n visual .github/workflows/ui.yml` -> :148-173/:58; `ls ui/scripts/visual-run.mjs` + :46/:55 (config + VISUAL_MODE wiring); baseline counts: baselines/linux 216 files, 1280x720-1x-dark 41 (25 routes + 16 accent samples), 1280x720-1x-light 25 |

## 1. One-paragraph verdict
A tight, well-commented visual-diff config: tolerance values match the spec doc (threshold 0.1 perceptual YIQ, maxDiffPixelRatio 0.001, animations disabled, caret hidden, css scale), per-platform baseline template `baselines/{platform}/{arg}{ext}` exactly matching the committed tree, one-retry flakiness policy with the fail-twice rationale, three documented chromium determinism flags (font-render-hinting=none, force-color-profile=srgb, disable-lcd-text — rounds 3/4/4c with run numbers), strict CI-only server non-reuse, and a 240s webServer budget with the incident run that motivated it. The only defect found is a stale case-count comment in the workflow that invokes it.

## 2. What actually works (verified)
— verified by command: `ls` of baselines/linux -> 216 files; PR-lane set (1280x720 @1x dark+light) = 25+25 = 50 route baselines, exactly the current 25-route manifest (manifest.ts:61-92), plus 16 accent-sample baselines — the snapshotPathTemplate this config defines resolves to precisely this layout. — CI wiring verified by rg: `node scripts/visual-run.mjs --mode=pr` runs on every PR (ui.yml:172-173); nightly at 03:41 UTC (ui.yml:58). Execution itself UNVERIFIED here (no playwright in this environment; baseline has no visual-lane run — environment limit, not a repo defect).

## 3. Findings
F-001 [S4] (cross-ref — the defect lives in .github/workflows/ui.yml, not this file)
Where: .github/workflows/ui.yml:140,149
Code:
```
  # 1280x720 @1x, dark + light — 15 routes × 2 modes = 30 cases against the
  name: visual (PR-lite lane — 30 cases vs pinned Linux baselines)
```
What is wrong: the comment and job name claim 30 cases (15 routes × 2) — stale by two route expansions; manifest.ts:18-22 (w26.k20) grew ROUTES to the 25-route truth, so the lane now runs 50 cases per invocation. Why it matters: operators reading the workflow under-budget the lane by 40% and misread coverage claims; also contradicts manifest.ts:29 ("PR lane = 25 routes × {dark, light} = 50"). What correct looks like: update both lines to 25 routes × 2 = 50. Confidence: HIGH — Verification: ran (baseline file counts: 25 dark + 25 light PR set on disk).

## 4. Per-function table
File <100 LOC — the single defineConfig block covers: testDir/timeout/parallel/retries/reporter (:12-24), snapshotPathTemplate + expect tolerances (:25-43), use/launchOptions determinism flags (:44-70), webServer (:71-90).

## 5. Test honesty
N/A (config). The determinism flags' provenance comments cite concrete CI run numbers (43384519508, 43400611004, 43410853384) — consistent story across rounds 3/4/4c; UNVERIFIED (cannot re-run CI from here).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| tolerance rationale (0.1 / 0.001) | config :30-38 + visual.spec.ts:96-101 | REAL | identical values in both files |
| per-OS baseline reality, Linux canonical | config :7-8 | REAL | baselines/linux/ is the only platform dir on disk |
| PR-lite lane count "30 cases" | ui.yml:140,149 | FALSE (stale) | F-001 — 50 today |

## 7. Dead / duplicated / unreachable code
None. `fullyParallel: true` is safe here (each case is an isolated page/context; committed provenance "57.8s" for the lane, ui.yml:141-143).

## 8. Dependencies & flags
@playwright/test defineConfig only; webServer reuses the 5173 dev server with strictPort + CI-never-reuse pattern.

## 9. Open questions for the next agent
The nightly lane (VISUAL_MODE=nightly, 216-case grid incl. 2x sets) has no committed execution log in-tree; when a nightly artifact surfaces, verify the 2x baselines (this run audited 3 of them — consistent).
