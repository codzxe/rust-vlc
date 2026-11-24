# AUDIT: ui/src/components/form/TextField.tsx (compact, 55 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 55 / 1 (TextField) |
| File verdict | REAL (correct a11y wiring; one spread-order hazard) |
| Reachable from shipped entry point? | YES — SettingField.tsx:254 renders it for string settings rows (the privacy lastfm fields, path inputs, etc.); colocated TextField.test.tsx in vitest baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read of full file; consumer check (SettingField:254) from the privacy audit |

## 1. One-paragraph verdict
A real, small, correct primitive: visible label wired by htmlFor/id, the
w26.k01 hideLabel mechanism (aria-label on the input, no text node — the
one-mechanism rule states.md claim 12 pins), error state switching
aria-invalid + aria-describedby onto a rendered error paragraph. The only
defect is spread ordering: `{...rest}` lands AFTER the computed
aria-describedby/aria-label (:43), so any caller passing those through
rest silently disconnects the error announcement — exactly the kind of
attribute a host adds for its own hint text.

## 2. What actually works (verified)
— a11y chain: error -> aria-invalid=true + aria-describedby=<id>-error ->
rendered <p id>. Pinned by TextField.test.tsx:20-31 (from the states.md
census, baseline green).
— forwardRef + displayName correct; id override supported.

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/TextField.tsx:41-43
Code:
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : undefined}
          {...rest}
What is wrong: rest is spread last — a caller-supplied
`aria-describedby` (hosts that append their own hint id) REPLACES the
error wiring instead of merging, and a caller `aria-label` overrides the
hideLabel label. No current caller does this (spot-check of SettingField),
so it is a hazard, not an active bug. What correct looks like: merge
describedby ids, or spread rest first and compute aria-* after.
Confidence: HIGH — Verification: read.

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| TextField | 23-53 | REAL | S4 F-001 |

## 5. Test honesty
TextField.test.tsx (8 cases from the states.md census) — error-state test
would fail if the a11y chain broke. No test covers rest-override
interactions (F-001 is untested surface).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Error state wires aria-invalid + aria-describedby to the rendered error message" | :20-21 | REAL | :41-48 + test :20-31 |
| "token seams pending the input tokens" | :19-20 | REAL | tokens.css has no input tokens (verified in tokens.css audit) |

## 7. Dead / duplicated / unreachable code
None. (No password input variant exists — recorded as the privacy-area
finding F-002 there, not double-counted.)

## 8. Dependencies & flags
form.css only. No flags.

## 9. Open questions for the next agent
- None for this file. The lastfm-password plain-text rendering is a
  schema/SettingField concern (privacy.tsx audit F-002).
