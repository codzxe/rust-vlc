# AUDIT: ui/src/components/form/RadioGroup.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 97 / 1 component |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: exported from form/index.ts; consumed by Dropdown/SegmentedControl/SwatchGroup family and settings screens (import chain verified by rg) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumer chain (exit 0); baseline ui-test.txt (form suites pass) |

## 1. One-paragraph verdict
Solid accessible primitive: native `type="radio"` inputs under one generated name (platform checked/Space semantics), `role="radiogroup"` fieldset with aria-label/aria-invalid, roving tabindex, and the shared arrow-key hook (wrap-around, disabled skipped). One S4 keyboard edge: when the controlled `value` matches no option, every input gets `tabIndex={-1}` and the group has no tab stop at all.

## 2. What actually works (verified)
— Consumer chain: rg shows Dropdown.tsx / SegmentedControl.tsx / SwatchGroup.tsx family + form.css grammar; baseline form tests pass.
— Controlled correctness: `checked={checked}` + `onChange` only fires on unchecked -> no double-fire.

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/RadioGroup.tsx:83-84
Code:
```
                // roving tabindex (doc 08 §4.5): only the selected option is tabbable
                tabIndex={checked ? 0 : -1}
```
What is wrong: if `value` matches no option (parent state drift, empty-string initial), no radio is tabbable — keyboard users cannot enter the group at all. The standard fix is selected-else-first-enabled. Blast radius: small (parents pass matching values), but it is a silent keyboard lockout. Confidence: HIGH — Verification: read.

## 4. Per-function table
N/A (<100 LOC): single component, all branches reviewed (disabled group, per-option disabled, invalid state, controlled checked).

## 5. Test honesty
Form tests (baseline pass) cover keyboard + selection; the no-matching-value tab-stop case is untested (F-001 invisible to the suite).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Options are native `type="radio"` inputs ... platform" | L30-32 | TRUE | L76-89 |
| "four arrows, wrap-around, disabled options skipped" | L47-48 | TRUE (delegated) | useRadiogroupKeyboard (shared hook, own tests) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
useRadiogroupKeyboard shared hook; form.css. No gates.

## 9. Open questions for the next agent
- None.
