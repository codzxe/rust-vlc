# AUDIT: ui/src/components/form/NumberField.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 132 / 3 (NumberField, clamp, stepBy) |
| File verdict | REAL |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (consumers: OnboardingScreen.tsx playback/subtitle steps, settings/SettingField.tsx int/float/duration rows — rg verified; both routes live). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg NumberField consumers (OnboardingScreen, SettingField); NumberField.test.tsx survey 5-86; read clamp/stepBy math by hand |

## 1. One-paragraph verdict
A correct, well-tested form primitive: native number input with min/max/step
forwarded (keyboard/pointer parity — j01-1), steppers that clamp and disable
at bounds, honest no-op on empty/non-numeric values, numeric-string
controlled values handled (j01-11), error a11y wiring complete. The single
defect is cosmetic-honesty: when the field is empty, both steppers render
ENABLED but silently no-op — the exact "control that looks alive" pattern
this codebase's own doctrine forbids.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): NumberField.test.tsx
executes labelled render, step+clamp, bounds-disable, native attr parity,
omitted-when-absent bounds, numeric-string stepping, error a11y, disabled
group, DOM snapshot.
— verified by hand: clamp ordering (min raise then max lower) is correct for
min ≤ max; `next === numeric` guard prevents duplicate onChange at the
bound; j01-11's parsed logic (44-46) handles number / numeric-string /
empty-or-garbage → undefined correctly.
— verified by read: the synthetic stepper event's honest-gap (no name/
isTrusted) is documented at the props level AND in form/states.md — callers
are warned to read the value.

## 3. Findings
### F-001 [S4] Empty field renders enabled steppers that silently no-op
Where: ui/src/components/form/NumberField.tsx:65-66 and 100/113
Code:
```
    const atMin = typeof min === "number" && numeric !== undefined && numeric <= min;
    const atMax = typeof max === "number" && numeric !== undefined && numeric >= max;
```
(and `disabled={disabled || atMax}` at 100, `disabled={disabled || atMin}`
at 113)
What is wrong: When the field is empty (numeric === undefined), atMin/atMax
are both false → both steppers enabled; stepBy's first line (56) then
no-ops. The user clicks a healthy-looking stepper and nothing happens, with
no feedback.
Why it matters: Contradicts the codebase's own no-dead-controls doctrine
(extensions.tsx header 26-30) at the primitive level; a disabled stepper
with the honest "no value" state would be self-documenting.
What correct looks like: `disabled={disabled || numeric === undefined ||
atMax}` (and symmetric for down) — or seed from min on empty-step, if the
design wants an affordance.
Confidence: HIGH — Verification: read + hand-trace; test 53 pins the no-op
BEHAVIOR but not the enabled-looking state (asserts onChange not called,
not button disabledness).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NumberField | 35-130 | REAL | S4 (F-001) | forwardRef + displayName; error id wiring correct; labelHidden aria-label path correct |
| clamp | 48-53 | REAL | — | correct order; undefined bounds skipped |
| stepBy | 55-63 | REAL | — | synthetic event cast documented; no-op guards correct; disabled guard duplicated (safe) |

## 5. Test honesty (NumberField.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| labelled render + steppers (6-14) | YES | structure |
| step + clamp through onChange (15-24) | YES | asserts clamped values |
| bounds disable (25-33) | YES | asserts disabled at min/max |
| native attr parity j01-1 (34-52) | YES | min/max/step on the input; absent when not provided |
| numeric string steps / empty no-ops (53-65) | YES | j01-11 regression pin |
| error a11y / disabled group / snapshot (66-86) | YES | aria wiring + inert group |
Mutation check: remove the clamp → test 15 fails. Remove the numeric-string
parse → test 53 fails. Gap: no assertion on stepper disabledness for the
empty field (F-001 untested).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Steppers clamp to min/max and report through the standard onChange; steppers disable at the bounds" | doc comment 26-29 | REAL | code + tests |
| "min/max/step are forwarded to the native input … keyboard users get the same contract" | doc comment 29-33 | REAL | test 34 |
| synthetic-event honest gap (no name/isTrusted) | props doc 17-22 + states.md | REAL | code 59-62 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
react only + form.css. No feature flags. Consumed by onboarding + settings
generated fields (rg).

## 9. Open questions for the next agent
- Should the steppers seed from min/max on an empty field (VLC behavior
varies)? Product call; current no-op is defensible but should be visibly
disabled.
