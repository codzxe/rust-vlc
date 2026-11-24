# AUDIT: ui/src/components/form/SwatchGroup.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 106 / 2 (SwatchGroup + DEFAULT_SWATCHES) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — AccentColorPicker (settings/appearance) + OnboardingScreen (route /onboarding) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "SwatchGroup" src` (2 production consumers + tests); `rg -c "it\(" SwatchGroup.test.tsx` (13) |

## 1. One-paragraph verdict
Clean radiogroup swatch picker: correct roving tabindex (only checked swatch tabbable), shared arrow-key grammar via useRadiogroupKeyboard (wrap-around, disabled skipped), per-swatch disabled, 13 tests. The frame's 9-accent order matches the generated AccentId union. The onChange reaches the theme system through AccentColorPicker (settings persistence -> data-accent on :root per tokens.css). Zero findings.

## 2. What actually works (verified)
— verified by consumer sweep (AccentColorPicker.tsx, OnboardingScreen.tsx) and SwatchGroup.test.tsx (13 cases). Baseline vitest 1705 passed.

## 3. Findings
None. Checked: roving tabindex, aria-checked, click only when unchecked, disabled merging, id generation.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DEFAULT_SWATCH_ORDER/SWATCHES | 36-48 | REAL | — | frame order; labels overridden by screens |
| SwatchGroup | 58-106 | REAL | — | radiogroup semantics |

## 5. Test honesty
13 tests (would fail on roving/keyboard/checked regressions — suite pattern shared with RadioGroup).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "frame: orange first, then red, yellow, green, teal, blue, purple, magenta, gray" (L31-33) | comment | REAL (modulo naming: teal=cyan, purple=violet, magenta=pink) | DEFAULT_SWATCH_ORDER L36-46 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Generated AccentId type (tokens/generated/tokens) — the generated-vs-handwritten boundary is a type import, no drift risk (compiler-checked).

## 9. Open questions for the next agent
- None.
