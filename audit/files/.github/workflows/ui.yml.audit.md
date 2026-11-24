# AUDIT: .github/workflows/ui.yml
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 288 / 4 jobs (ui, visual, visual-nightly, visual-baseline-seed), 5 gate steps + caches/uploads |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CI workflow file; triggers on push/PR to main, nightly cron `41 3 * * *` (L52-60). Not part of the release binary lane; its subject (ui/) is the desktop product's UI. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 json scripts-check (exit 0, all 6 scripts present); ls baselines (216 files); grep manifest/MATRIX (counts); grep ci.yml UI-duplication (none) |

## 1. One-paragraph verdict
This workflow is real, correctly structured, and honest about its one soft spot (the non-blocking Windows/macOS baseline-seed job is explicitly labeled "[BLOCKED on first real run]" with continue-on-error — a captured-not-faked stance). Gate steps genuinely fail (`if: always()` is keep-going aggregation, not `|| true`; no continue-on-error on any gate step). Every referenced npm script exists in ui/package.json, the visual driver `ui/scripts/visual-run.mjs` exists and propagates Playwright's exit code verbatim. The one real defect: the file's case-count comments (30-case PR lane, 136-case nightly) are stale — the tree's current visual manifest defines 50 PR cases / 216 nightly cases, and those stale numbers are baked into the job `name:` strings shown on every CI run page.

## 2. What actually works (verified)
— verified by `python3` json read of ui/package.json -> typecheck `tsc --noEmit`, lint `eslint .`, lint:css `node scripts/lint-css.mjs 'src/**/*.css'`, l10n:check `node scripts/l10n-check.mjs`, test `vitest run`, test:e2e `playwright test` all present (exit 0).
— verified by `ls ui/scripts/visual-run.mjs` + read: driver maps `--mode=pr|nightly` and `--update` onto `playwright test --config playwright.visual.config.ts [--update-snapshots]`, `process.exit(child.status ?? 2)` — honest exit propagation.
— verified by `ls ui/e2e/visual/baselines/linux/ | wc -l` -> 216 baseline PNGs (matches the current 216-case full matrix in manifest.ts).
— verified by `grep -n "npm run typecheck\|vitest\|playwright" .github/workflows/ci.yml` -> no UI lane forked into ci.yml (matches header claim "the UI lane is NOT forked into ci.yml").

## 3. Findings
F-001 [S3]
Where: .github/workflows/ui.yml:29-34, 141-143, 149, 201-206
Code:
```
#     `visual` = PR-lite lane (1280x720 @1x, dark+light — 15 routes × 2
#     modes = 30 cases; committed Linux baselines re-pinned for Solstice
#     v2 + wave-3 by issue #37, proven green 2026-11-09: 30 passed 57.8s
```
and
```
  visual:
    name: visual (PR-lite lane — 30 cases vs pinned Linux baselines)
```
and L205-206:
```
  visual-nightly:
    name: visual (full matrix — 136 cases, nightly)
```
What is wrong: The counts are stale. Current tree truth (ui/e2e/visual/manifest.ts, w26.k20 note): "**Full matrix = 25 × 2 × 2 × 2 + 16 = 216 nightly cases; PR lane = 25 routes × {dark, light} = 50.**" and ui/e2e/visual/MATRIX-20261113.md:41: "**25 routes** (was 15) and the nightly grid **216 cases** (was 136)." The stale "30 cases"/"136 cases" appear in job `name:` fields, which render on the CI run page — the visible lane description misstates the lane size by 40-66%. Why it matters: a reviewer eyeballing a red nightly run is told "136 cases" while 216 ran; per this audit's spec, names that lie are findings. What correct looks like: update the two `name:` strings and the header comments to 50/216. Confidence: HIGH — Verification: read + grep of manifest/MATRIX docs.

F-002 [S4]
Where: .github/workflows/ui.yml:43-48
Code:
```
# Action pins (re-resolved via `git ls-remote` on 2026-11-09, issue #5;
# no drift vs the 4.A1 table):
#   actions/checkout        v7.0.1  3d3c42e5aac5ba805825da76410c181273ba90b1
```
What is wrong: nothing in-file — all four `uses:` lines do pin full 40-hex SHAs (L81, 83, 90, 132, 153, 155, 162, 184, 193, 211, 213, 220, 235, 264, 266, 282). The pins' correspondence to the named tags cannot be re-verified from this sandbox. Confidence: HIGH that pins are SHA-pinned — Verification: read; tag->SHA mapping UNVERIFIED (no network access to GitHub refs).

## 4. Per-function table (per job/step)
| Job / step | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `on:` triggers | 52-60 | REAL | — | push/PR main + cron 41 3 * * * + dispatch |
| `permissions` / `defaults` / `concurrency` | 62-73 | REAL | — | contents: read; wd=ui; cancel-in-progress only for PRs — correct |
| job `ui` cache steps | 81-95 | REAL | — | checkout/setup-node@SHA/cache keyed on ui/package-lock.json |
| "1. typecheck" | 98-100 | REAL | — | `npm run typecheck` exists; if: always() = keep-going, can fail |
| "2. lint" + lint:css | 102-111 | REAL | — | stylelint gate chained after wave-L round 4 gap — real fix, comment documents why |
| "3. l10n catalog check" | 116-118 | REAL | — | script exists (l10n-check.mjs) |
| "4. unit tests (vitest)" | 120-122 | REAL | — | baseline: vitest 1705 passed corroborates |
| "5. e2e smoke" | 124-128 | REAL | — | chromium install + `playwright test`; can fail |
| Upload Playwright report | 130-136 | REAL | — | if: always(), retention 7d |
| job `visual` | 148-198 | REAL | S3 | compare vs committed Linux baselines via visual-run.mjs --mode=pr; diff-image upload if: failure() — evidence on red |
| job `visual-nightly` | 205-240 | REAL | S3 | gated `if: schedule/dispatch` only; 25 min timeout |
| job `visual-baseline-seed` | 253-287 | REAL | — | continue-on-error + BLOCKED label — honest non-blocking capture lane; `--mode=nightly --update` maps to --update-snapshots (verified in driver) |

## 5. Test honesty
No tests in this file. Honesty of referenced lanes: PR/nightly compare steps have no soft-fail; the only non-blocking job is explicitly labeled blocked-by-environment with a fail-by-design rationale (L39-41: "any compare-mode run on that OS fails by design (missing baselines) — that is honest, not fake green").

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "15 routes × 2 modes = 30 cases" | ui.yml:29-31, 141-143 | FALSE-on-current-tree (stale) | manifest.ts: PR lane = 50; MATRIX-20261113.md:41 |
| "136 cases" nightly | ui.yml:34, 206 | FALSE-on-current-tree (stale) | manifest.ts: 216; MATRIX-20261113.md:51, 66-67 (216/216 re-generated, 50 passed) |
| "30 passed 57.8s ... provenance in MATRIX-20261109.md" | ui.yml:31-32 | REAL (historical) | MATRIX-20261109.md:63 "PR lane ... 30 passed (48.4 s)"; :98 "30/30 (57.8 s)" — both figures appear in the provenance doc |
| "the UI lane is NOT forked into ci.yml" | ui.yml:2-3 | REAL | grep ci.yml -> no UI jobs |

## 7. Dead / duplicated / unreachable code
None. No duplicated UI lane in ci.yml. No `|| true`, no continue-on-error on gates.

## 8. Dependencies & flags
Actions pinned by commit SHA (checkout v7.0.1@3d3c42e5…, setup-node v6.0.0@2028fbc5…, cache v4.3.0@0057852…, upload-artifact v7.0.2@cf430e03…). node-version 24. npm cache keyed on lockfile; Playwright browser cache keyed on `hashFiles('ui/package-lock.json')` — correct relative path from repo root.

## 9. Open questions for the next agent
- Tag->SHA validity of the four action pins: UNVERIFIED (no network).
- Does the nightly cron actually fire (phase7-exit.md §6 says CI Actions billing-blocked, "every run dies at 0 steps")? If billing is still blocked, this whole workflow is dead-on-arrival in CI and all its guarantees are local-only. UNVERIFIED from sandbox.
