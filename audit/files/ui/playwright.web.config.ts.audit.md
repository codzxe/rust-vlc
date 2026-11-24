# AUDIT: ui/playwright.web.config.ts
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines | 69 |
| File verdict | REAL (config-driven Playwright lane config; matrix.config.json cross-verified) |
| Reachable from shipped entry point? | NO — e2e lane config (via `npm run webmatrix` → scripts/web-matrix-run.mjs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; cat matrix.config.json; rg wiring; baseline check |

## 1. One-paragraph verdict
Playwright config for the browser-matrix lane: reads EVERY knob from ui/e2e/web/matrix.config.json (timeoutMs 240000 validated positive-integer at :44-49, launchArgs `--no-sandbox --disable-dev-shm-usage`, executable via env `WEB_MATRIX_CHROMIUM`), deliberately declares NO webServer/baseURL so a stray direct invocation fails fast in the spec rather than silently testing the wrong origin (:4-12 comment + verified behavior claim). testDir ./e2e/web, testMatch web-matrix.spec.ts, workers 1 sequential (conformance measurement rationale :55-57). Cross-check: matrix.config.json values match what this file reads (read both). Baseline: the lane is NOT run in any committed baseline run file (rc-smoke.txt has web-ui-build + browser-load only; ui-test.txt is vitest) — the evidence is the committed local report docs/web/browser-matrix-20261110.md(+json).
## 2. What actually works (verified)
- matrix.config.json parse + validation logic read; knobs match.
- Wiring: ui/package.json:16 `"webmatrix": "node scripts/web-matrix-run.mjs"`.
## 3. Findings
F-001 [S3]
Where: this lane vs audit/runs/baseline/
What is wrong: no committed automated run of the webmatrix lane (evidence = one dated local report, 2026-11-10, produced with an env-overridden "agent-browser Chrome for Testing 157.0.8103.50" per matrix.config.json's own comment). Same billing-blocked CI posture as the visual/soak lanes. Confidence: HIGH — Verification: ran (rg baseline + ls docs/web).
## 4. Test honesty
N/A (config). The fail-fast design (no silent wrong-origin run) is the honesty mechanism — verified only by reading the spec's error path; the spec file itself is another agent's assignment.
## 5. Doc claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this config declares no webServer and no baseURL on purpose" | file :8-11 | REAL | :51-69 defineConfig has neither key |
| knobs read from matrix.config.json, "never a literal in code" | matrix.config.json comment | REAL | only literals here are structural (workers:1, testMatch) |
