# AUDIT: ui/src/screens/playlist/subtitles/harness.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 91 / 1 describe, 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:149 "playlist/subtitles/harness.test.tsx (2 tests)" ✓ PASS). Dev-only harness by design. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg suite in baseline ui-test.txt |

## 1. One-paragraph verdict
The subtitle-flow vision-harness twin of the visualizer harness test: a crash smoke over five frame-state compositions (folder pairs / multi / online, sub-filename with the dropdown opened, subtitle rows) and an env-gated HTML exporter (W26B11_HARNESS_OUT) that inlines 9 real CSS files, opens the indexed dropdown trigger, clicks the frame's selected rows, and serializes the body. Honest dev-lane scoping; the same two hygiene defects as its sibling.

## 2. What actually works (verified)
- Baseline ui-test.txt:149: `✓ src/screens/playlist/subtitles/harness.test.tsx (2 tests) 197ms`.
## 3. Findings
F-001 [S4] :63-65 — env-gated export test early-returns with ZERO assertions in the default lane (vacuous green in CI; the documented w26 dev-lane convention). Confidence: HIGH — Verification: read.
F-002 [S4] :55-61 — smoke-only first test (`innerHTML.length > 0`); acceptable, honestly scoped. Confidence: HIGH — Verification: read.
## 4. Per-function table
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| crash smoke over 5 states | 55-61 | REAL (smoke) | F-002 |
| env-gated export | 63-90 | REAL (dev-lane) | F-001; real CSS inlined; frame pre-selection clicks |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crash smoke | YES only for crashes | render throws |
| export | NO in default lane | F-001 |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "see the lane log for the loop record" | :4-5 | UNVERIFIED | w26 lane log not located |
## 7. Dead / duplicated / unreachable code
None beyond the env-gated branch.
## 8. Dependencies & flags
@testing-library/react, node:fs. Env knob W26B11_HARNESS_OUT (dev only).
## 9. Open questions for the next agent
- Same skipIf conversion recommendation as the visualizer harness (F-001).
