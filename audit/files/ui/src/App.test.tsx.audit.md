# AUDIT: ui/src/App.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A01 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 243 / 12 (2 describes) |
| File verdict | REAL (baseline-verified green; the app-level routing/gate/hydration contract) |
| Reachable from shipped entry point? | YES — tests App.tsx itself, the root component of the shipped UI |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline ui-test.txt:69-74 |

## 1. Verdict
The root routing contract suite: first-run gate over 13 shell routes, lazy-chunk screens awaited
with findBy, settings schema redirect, 404, ?onboarding=1 per-navigation bypass semantics, and
provider-stack theme/accent hydration asserted on document.documentElement.dataset. Baseline
green: ui-test.txt:69 `✓ src/App.test.tsx (12 tests) 1898ms` (8 routing + 4 hydration).

## 2. What actually works (verified)
- Baseline 12/12 (cite above), no skips recorded.
- Mutation reasoning: gate regressing to player-only fails the 13-route loop (:70-90);
  lazy screens not awaited would throw synchronously; bypass persisting state fails
  (:160-167); light-theme hydration regression (e03-F1) fails via dataset.theme assertion
  (:201-208); accent-only hydration keeping dark theme fails (:230-242).
- Mock posture is honest: MockTransport at the client seam (:14), real router, real App.

## 3. Findings
F-001 [S4]
Where: ui/src/App.test.tsx:51-59, 138-145 vs :193-199
Code:
    window.localStorage.clear();
    resetOnboardingForTests(); // leave a fresh run for the later route tests
What is wrong: the first describe seeds/clears localStorage inline per test with hand-written
"leave a fresh run for the later route tests" comments — inter-test ordering is a silent
dependency (vitest runs describes sequentially by default, but any shuffle/parallel change
breaks tests that assume a fresh run). The second describe has the proper afterEach (:193-199);
the first does not. Hygiene, not dishonesty: each test still asserts real behavior.
What correct looks like: one afterEach in the first describe too, removing ordering assumptions.
Confidence: HIGH — Verification: read.

## 4. Per-test table (file >100 LOC)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| first run / → onboarding gate | YES | h1 text exact |
| /player/music variant | YES | data-variant="music" attribute |
| gate guards whole app (13 routes) | YES | loop of exact h1 per path |
| settings schema redirect | YES | General aria-current=page |
| onboarding route | YES | h1 exact |
| 404 catch-all | YES | "Page not found" |
| completed / lands on player | YES | h1 Player |
| ?onboarding=1 bypass semantics | YES | 5 sub-assertions incl. re-gate after param |
| light hydration ×3 | YES | dataset.theme=="light" per route |
| accent hydration | YES | dataset.accent=="violet" + theme stays dark |

## 5. Test honesty
No tautologies; every assertion is DOM-observable state. Lazy-screens use findBy (the honest
seam per w26.k20 comment :61-64) instead of sync getbys that would false-pass on empty paint.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every route renders a real screen" (route smoke header) | :22-26 | REAL | 13-route loop + 4 more route tests |
| gate wraps EVERY AppShell route (w26.i20) | :65-90 | REAL | asserted on 13 paths |
| "?onboarding=1 … per navigation only" | :147-168 | REAL | post-param re-gate asserted |

## 7. Dead / duplicated / unreachable code
None; all helpers used.

## 8. Dependencies & flags
MockTransport (client seam mock), ThemeProvider, MemoryRouter; no snapshot tests; no fakes of App.

## 9. Open questions for the next agent
None blocking.
