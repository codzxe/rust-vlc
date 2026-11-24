# AUDIT: scripts/rc-smoke-web.mjs
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 93 / 1 (top-level flow: preflight, launch, seed, boot-check, pageerror gate) |
| File verdict | REAL (executed by the orchestrator baseline: PASS) |
| Reachable from shipped entry point? | YES as tooling — scripts/rc-smoke.sh (step 8, web-ui-browser-load) builds ui/, serves vite preview, runs THIS script |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
Real browser smoke, executed for real in the orchestrator baseline: `audit/runs/baseline/rc-smoke.txt:205-210` shows this script loading the production bundle over vite preview, booting to heading "Player", chromium 143.0.7499.4, 0 page errors, exit PASS. The playwright-pin-drift bridge (explicit executablePath instead of pinned revision lookup) is honestly documented with its failure mode. One weakness in the pass criterion: ANY non-empty `<h1>` passes — an app that boots into an error surface with an h1 would pass the boot check (the pageerror gate and the printed heading text are the real safety nets).

## 2. What actually works (verified)
— Baseline citation (executed by orchestrator): rc-smoke.txt:206-210 — "loaded http://127.0.0.1:4173/", 'level-1 heading: "Player"', "chromium: 143.0.7499.4", "PASS (loaded, booted to the app surface, 0 page errors)".
— Exit-code contract implemented: 2 usage/preflight (L31-34, L43-46), 1 fail (L92), 0 pass (L88) — matches the header claim.
— `addInitScript` seeds `lumen.onboarding.v1` BEFORE any app script runs (L61-69) — the claimed seam parity with ui/e2e/smoke.spec.ts is real (same key seen in the phase-4 smoke-fix narrative).
— pageerror gate: any uncaught page error fails the run (L57, L82-85).
— Failure path closes the browser (`browser?.close().catch(() => {})`, L91).

## 3. Findings

F-001 [S3]
Where: scripts/rc-smoke-web.mjs:73-77
Code:
```
  const heading = await page.waitForSelector("h1", { timeout: 10_000 });
  const headingText = (await heading.textContent())?.trim() ?? "";
  if (!headingText) {
    throw new Error("boot check failed: the app rendered no level-1 heading");
  }
```
What is wrong: the boot check asserts only that SOME h1 exists with non-empty text. A catastrophic app that renders an error boundary with `<h1>Something went wrong</h1>` passes this check (the header claims it proves "boots ... to the player surface"). Why it matters: the claimed guarantee is stronger than the implemented one; the operator-visible printed heading (baseline: "Player") is the actual quality bar. What correct looks like: assert the expected player-surface heading (like ui/e2e/smoke.spec.ts does post-4.F1: "Player"). Confidence: HIGH — Verification: read + baseline output shows the strong form happened empirically, not by enforcement.

F-002 [S4]
Where: scripts/rc-smoke-web.mjs:54
Code:
```
    args: ["--no-sandbox"],
```
What is wrong (context-dependent): --no-sandbox is required in CI containers but would be wrong for a user-facing tool; this is a CI smoke, so acceptable — noted because it lowers the browser's own security posture while loading the app. Confidence: HIGH (read); severity S4 (scoped to CI).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| preflight/usage | 30-46 | REAL | - | exit 2 paths; require-bridge from ui/ with actionable message |
| launch+seed+goto+wait | 48-79 | REAL | S3 | F-001: h1 check too weak |
| pageerror gate + exits | 82-93 | REAL | - | honest 0/1 exit; browser cleanup |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| this smoke (baseline run) | PARTIAL | a broken-but-rendering app with an h1 and no uncaught errors PASSES (F-001); a JS-crashing app FAILS (pageerror gate); a no-h1 app FAILS |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the built SPA loads over HTTP (no dev server, no HMR)" | L7-8 | REAL | baseline run through vite preview (rc-smoke.txt:205) |
| "a fresh profile boots through the first-run onboarding gate to the player surface" | L8-10 | SHALLOW->REAL empirically | F-001: enforced as "some non-empty h1"; empirically "Player" (baseline) |
| "ZERO uncaught page errors" | L11 | REAL | pageerror listener + fail (L57/82-85); baseline shows 0 |
| "if the revision gap ever grows past what the CDP protocol tolerates, this script fails loudly here first" | L18-20 | REAL-UNPROVEN | design read; the gap (1248 vs 1200) worked in the baseline run |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@playwright/test resolved from ui/ (createRequire over ui/package.json — correct for a scripts/-side tool); node >= 14 (import.meta.dirname needs node 20.11+ — the sandbox runs node 24 per phase-4 gate log; UNVERIFIED on older CI images, none claimed).

## 9. Open questions for the next agent
1. scripts/rc-smoke.sh — does it pass the chromium executable it discovers, and does the overall FAIL verdict (baseline) trace to sections other than browser-load? (baseline says browser-load PASS, overall FAIL.)
