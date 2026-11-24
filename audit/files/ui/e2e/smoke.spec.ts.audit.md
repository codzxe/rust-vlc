# AUDIT: ui/e2e/smoke.spec.ts
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 65 / 3 |
| File verdict | REAL-UNPROVEN — honest design (seeded boot, unseeded gate redirect, full wizard walk with exact persisted-store assertion); no committed baseline run of the playwright lane exists |
| Reachable from shipped entry point? | NO — Playwright e2e; CI lane .github/workflows/ui.yml (typecheck/lint/vitest/playwright job) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline (playwright: 0 hits — no e2e run evidence); read spec |

## 1. One-paragraph verdict
Three honest browser tests: (1) the seeded-wizard-complete boot must land the
player heading (localStorage seed through the same OnboardingState seam the
visual harness uses); (2) a FRESH profile hitting a deep link
(/settings/global) must be redirected into the wizard with the gate visible —
the browser-only regression class (store hydration vs router commit order)
that previously passed every committed e2e, as the header itself admits
(L38-40); (3) the full 8-step wizard walk (Next ×7 + Get started) must land
on /player AND persist the exact `lumen.onboarding.v1` JSON shape asserted
field-for-field from localStorage. The one gap is evidence, not design: no
committed baseline log shows a playwright run — the lane's greenness is
self-declared in the file header ("Green 2026-10-23 … chromium 156").

## 2. What actually works (verified)
— Read: all three tests assert URL/role/testid/exact-JSON outcomes; the
header documents its own history of going stale (P4-GATE) and how it was
fixed by seeding — honest maintenance narrative.
— rg over audit/runs/baseline/*.txt for "playwright" → 0 hits: the e2e lane
is NOT part of the committed baseline evidence set (vitest/typecheck/build
are; the orchestrator did not run playwright). UNVERIFIED: actual pass state.

## 3. Findings
F-001 [S4]
Where: ui/e2e/smoke.spec.ts:4-5
Code:
// Smoke: the dev shell boots and the routing skeleton lands on the player
// placeholder. Green 2026-10-23 (4.A1 closure) on chromium 156.0.8078.4;
What is wrong: the only greenness evidence for this file is its own comment
(a dated self-declaration); the committed baseline set contains no playwright
output (rg: 0 hits). The CI lane presumably runs it, but no in-repo log
proves a current green. Evidence hygiene for a test whose purpose is
regression protection.
Confidence: HIGH — Verification: rg baseline sweep.

## 4. Per-function table
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| boot lands on player | 21-33 | REAL-UNPROVEN | seed via addInitScript; heading assertion |
| first-run gate redirect | 35-45 | REAL-UNPROVEN | unseeded deep-link → /onboarding + testid |
| wizard completion | 47-65 | REAL-UNPROVEN | 7-step walk + exact persisted JSON equality |

## 5. Test honesty
Mutation reasoning: gate regression fails test 2 (URL + visibility); store
shape drift fails test 3's exact JSON equality; boot regression fails test 1.
Names match bodies. The header honestly records prior staleness and the
ad-hoc battery it replaces (L14-19).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Green 2026-10-23 … on chromium 156.0.8078.4" | file L4-5 | REAL-UNPROVEN | no committed playwright log; self-declaration only |
| "previously passed every committed e2e" (the gate gap) | file L39-40 | REAL (as history) | unit coverage existed (App.test.tsx redirect matrix cited); browser-level gap now covered by test 2 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@playwright/test; localStorage seam shared with visual harness (visual.spec.ts
cited in-file); no network beyond the dev shell.

## 9. Open questions for the next agent
- Should the orchestrator baseline include one `npx playwright test` run so
  e2e greenness has committed evidence? (Currently vitest-only evidence.)
