# AUDIT: ui/src/screens/onboarding/onboarding.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 925 / n.a. (pure stylesheet, ~50 class selectors) |
| File verdict | REAL-UNPROVEN (presentational; wired and selector-verified) |
| Reachable from shipped entry point? | YES — imported at screens/onboarding/OnboardingScreen.tsx:64 `import "./onboarding.css"`; OnboardingScreen is eager-imported by App.tsx:11 (`OnboardingEntry`) and rendered at App.tsx:296 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `grep -rn "onboarding.css" screens/onboarding/*.tsx`; class-usage sweep via python3 (50/50 selectors used); `sed -n '443,462p'` quote check |

## 1. One-paragraph verdict
Pure presentation for the first-run wizard. No logic, no fake data, no dead surface: every one of the 50 `.onboarding*` class selectors is referenced by OnboardingScreen.tsx (the three `--light/--dark/--system` modifiers are built via the template literal at OnboardingScreen.tsx:530 `onboarding__theme-card--${card.choiceId}` — initially flagged dead by static sweep, refuted by reading the TSX). Tokens are semantic (`var(--color-surface-deep)` etc.); raw hex appears only for frame-measured swatch depictions under an explicit `stylelint-disable color-no-hex` with provenance comments. Responsive (≤800px height, ≤760px width) and `prefers-reduced-motion` handled.

## 2. What actually works (verified)
— verified by python3 sweep: `total css classes: 50`, dead list after template-literal check = empty; import chain grep exit 0.

## 3. Findings
None. Checked for: dead selectors (none), unimported stylesheet (imported), raw-color escapes (all annotated `frame:` with stylelint pragma), z-index/selector hacks (none), `!important` abuse (none found in read).

## 4. Per-function table
n.a. — stylesheet, <100 "functions"; selector inventory captured above.

## 5. Test honesty
n.a. — no tests target CSS directly; visual harness owns geometry (not verifiable here).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Colors/fonts/radii strictly via semantic tokens ... raw px only for frame-measured structural values" | file header L18-20 | REAL | read of full file: only swatch/depiction hex + `frame:`-marked px values |

## 7. Dead / duplicated / unreachable code
None (see §1 sweep).

## 8. Dependencies & flags
Semantic tokens from tokens/generated/tokens.css; no feature flags.

## 9. Open questions for the next agent
None.
