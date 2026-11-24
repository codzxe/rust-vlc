# AUDIT: ui/src/components/form/Checkbox.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 40 / 1 (Checkbox) |
| File verdict | UNREACHABLE (real component, zero production mounts) |
| Reachable from shipped entry point? | NO — grep: no `<Checkbox` outside its own test file; only re-export at form/index.ts:7-8 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | grep "<Checkbox" (tests only); grep import sites (barrel re-export only); rg form.css classes (live) |

## 1. One-paragraph verdict
Well-built native-input checkbox primitive (forwardRef, useId, aria-invalid,
label htmlFor wiring, token-driven states in form.css:242+) that NOTHING in
the product renders. Its only consumers are its own tests
(Checkbox.test.tsx:9-35). The form barrel re-exports it (index.ts:7-8) — a
dead public export. The comment at PlaylistPanel.tsx:410 ("Checkbox face is
engine truth only") describes a different inline mechanism, not this
component.

## 2. What actually works (verified)
— Component mechanics are correct on read (id/label association, disabled
  class hook, invalid → aria-invalid + error class).
— form.css .lumen-choice styles exist (242-261+) — the skin is real; the
  component wearing it never ships.

## 3. Findings
F-001 [S3]
Where: ui/src/components/form/Checkbox.tsx:17-38 (whole component)
Code:
```
export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, invalid = false, disabled, id, className = "", ...rest }, ref) => {
```
What is wrong: Zero production call sites. The 40-LOC component + its test
suite + its barrel export ship as dead code. Tests pass while testing a
component no user can reach — the suite proves nothing about the product.
Confidence: HIGH — Verification: grep across src (only self-tests + barrel).
What correct looks like: either delete, or mount it where checkbox semantics
are needed (PlaylistPanel builds its own face instead).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Checkbox | 17-38 | REAL (code) / UNREACHABLE (product) | F-001 | native input + label |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Checkbox.test.tsx:9-35 | YES for the component in isolation | but the component is unreachable from any route — the suite validates dead code |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No checkbox export exists in the design package (doc 08 §1.5)" | :13-15 | REAL-UNPROVEN | design package not inspected; component exists regardless |

## 7. Dead / duplicated / unreachable code
The component itself (F-001). No duplication (PlaylistPanel's "checkbox face"
is an inline mechanism, not a copy).

## 8. Dependencies & flags
form.css, React. No flags.

## 9. Open questions for the next agent
form/index.ts owner: is the barrel re-export intentional staging for a future
screen, or leftover from a deleted consumer?
