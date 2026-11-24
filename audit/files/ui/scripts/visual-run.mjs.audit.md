# AUDIT: ui/scripts/visual-run.mjs
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 63 / 1 script |
| File verdict | REAL (thin, honest Playwright driver; env contract verified against the spec consumer) |
| Reachable from shipped entry point? | YES as a tool — npm scripts "visual"/"visual:pr"/"visual:nightly" (package.json:17-19) and the xtask visual lane spawn it. Not part of the shipped app bundle (dev/CI tooling). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg VISUAL_MODE/SEED_2PX consumers (e2e/visual/visual.spec.ts:30-31 — both read exactly as documented); ls playwright.visual.config.ts (exists); read package.json scripts |

## 1. One-paragraph verdict
A clean shim: maps CLI flags to Playwright's cli.js with env-var injection (the documented Windows-npm portability reason), rejects unknown arguments with exit 2, propagates Playwright's exit code verbatim ("stays honest" — and it does: `child.status ?? 2`), and gives a launch-failure hint. The env contract is real: visual.spec.ts:30-31 reads VISUAL_MODE/SEED_2PX exactly. One nit: it accepts `--update-baselines` as an undocumented alias for `--update` — the usage line doesn't mention it, so help and behavior drift by one flag.

## 2. What actually works (verified)
— rg: both env vars consumed by the spec (pr/nightly mode split + the 2px seeded-regression proof at visual.spec.ts:86).
— exit-code propagation path read and correct; spawn error path exits 2 with a hint.

## 3. Findings
F-001 [S4]
Where: ui/scripts/visual-run.mjs:33
Code:
```
  } else if (arg === "--update" || arg === "--update-baselines") {
```
What is wrong: `--update-baselines` is accepted but absent from the usage strings (L10, L39). Undocumented alias; harmless but the help lies by omission. Confidence: HIGH — Verification: read.

## 4. Per-function table
n/a (<100 LOC script).

## 5. Test honesty
The harness itself is the fidelity gate; rc-smoke's web-ui-browser-load (PASS) exercises a sibling lane; visual runs UNVERIFIED here (node_modules absent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exit code: Playwright's exit code is propagated verbatim" | visual-run.mjs:18-19 | TRUE | L63 `process.exit(child.status ?? 2)`; signal-kill maps to 2 |
| usage line lists 3 flags | visual-run.mjs:10 | INCOMPLETE | 4th alias exists (F-001) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@playwright/test's own cli (zero new npm deps — verified: only spawns existing path). No feature flags.

## 9. Open questions for the next agent
- None.
