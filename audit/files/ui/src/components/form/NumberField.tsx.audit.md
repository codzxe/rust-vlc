# AUDIT: ui/src/components/form/NumberField.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 132 / 1 (NumberField component + inner clamp/stepBy) |
| File verdict | REAL-UNPROVEN (sound primitive; synthetic stepper seam documented) |
| Reachable from shipped entry point? | YES — SettingField.tsx:297 (settings int rows), onboarding NumberField usage (OnboardingScreen.tsx:390 comment), form barrel |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; grep consumers |

## 1. One-paragraph verdict
Correct number input + pointer steppers: steppers clamp to min/max, disable at bounds, and fire the standard onChange via a DOCUMENTED synthetic event (jsdom lacks stepUp/stepDown; the honest-gap note tells consumers to read the value, not the event); min/max/step forward to the native input so keyboard arrows share the same bounds contract; numeric-string values parse before stepping (j01-11 fix); label/error/aria plumbing complete. No findings.

## 2. What actually works (verified)
— verified by read: clamp order (min then max), `next === numeric` no-op guard, atMin/atMax disable logic, aria-invalid/describedby error wiring, labelHidden aria-label path, forwardRef + displayName.

## 3. Findings
None. Checked: bounds math, no-value guard (NaN -> no fabricated step), synthetic event shape, stepper a11y (aria-hidden + tabIndex -1 with native-arrow alternative documented), controlled value passthrough, id/error id composition.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NumberField (clamp, stepBy inner) | 39-131 | REAL-UNPROVEN | — | see §2 |

## 5. Test honesty
Form-lane tests cover the stepper contract per the header's j01-1/j01-11 fix citations (UNVERIFIED depth — not my file list).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Steppers clamp to min/max ... steppers disable at the bounds (no fabricated out-of-range values)" | L23-25 | REAL | clamp + atMin/atMax disabled |
| "native arrows respect the bounds and the step" | L30-32 | REAL | min/max/step forwarded to input (L97-99) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
form.css only. No feature flags.

## 9. Open questions for the next agent
None.
