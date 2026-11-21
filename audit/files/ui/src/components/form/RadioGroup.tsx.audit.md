# AUDIT: ui/src/components/form/RadioGroup.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 97 / 1 (RadioGroup) |
| File verdict | REAL (code) — DEAD COMPONENT: zero production consumers; the pattern it exists to share is re-implemented inline elsewhere |
| Reachable from shipped entry point? | NO — not even in the dev/web build: rg `<RadioGroup` matches ONLY RadioGroup.test.tsx. Exported via components/form/index.ts but imported by no screen. (The whole ui tree is additionally unreachable as product, W14.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `<RadioGroup` (only the test file); rg `RadioGroup` outside components/form (SettingsPanel.tsx re-implements the pattern as PanelRadioGroup — 3 comment references, 0 imports); read useRadiogroupKeyboard.ts (the shared hook RadioGroup uses); read SettingsPanel.tsx:295-330 (the inline fourth copy of the roving logic); rg `useRadiogroupKeyboard` users (SwatchGroup + RadioGroup only) |

## 1. One-paragraph verdict
A well-built primitive (native radios + shared roving hook + fieldset semantics + disabled/error states) that nothing uses: its only consumer is its own 10-case test suite. The kicker is the reason it exists — useRadiogroupKeyboard's header says the roving logic "was copy-pasted verbatim ×3" and was unified into the hook — and SettingsPanel.tsx:275-330 ships `PanelRadioGroup`, a FOURTH implementation of the same arrow-ring logic that does not use the hook, while the shared primitive sits dead in the form barrel. The one code defect: when `value` matches no option, every radio gets `tabIndex={-1}` and the group becomes keyboard-unreachable (no Tab stop), violating the roving contract the component documents.

## 2. What actually works (verified)
— verified by reading l.51-95 → correct ARIA structure: fieldset/radiogroup, aria-label name, per-option label htmlFor wiring, native checked semantics, group `disabled` via the fieldset attribute (platform-native inertness), aria-invalid flag.
— verified by reading useRadiogroupKeyboard.ts → four-arrow wrap-around, disabled skipping, preventDefault ONLY on handled arrows (so native radio arrow behavior cannot double-fire onChange); RadioGroup's input `onChange` guard `if (!checked)` avoids re-firing on the already-selected option.
— verified by reading RadioGroup.test.tsx test count (10 cases) — the suite exercises the component it keeps alive.
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S3]
Where: ui/src/components/form/RadioGroup.tsx:34 (whole component) + components/form/index.ts (export)
Code:
```
export function RadioGroup({
  label,
  options,
  value,
  onChange,
```
What is wrong: Zero production consumers. rg `<RadioGroup` across ui/src matches only RadioGroup.test.tsx. The component is exported from the form barrel, styled by form.css (`.lumen-radiogroup*`/`.lumen-choice--radio`), tested by 10 cases + a snapshot — and mounted by no screen. Meanwhile SettingsPanel.tsx:275 ships `PanelRadioGroup`, an inline re-implementation of the same roving-radiogroup grammar (own keydown ring at :295-313, own tabIndex stop at :325), whose comments even cite "the RadioGroup primitive's pattern" — the exact copy-paste drift useRadiogroupKeyboard's header ("copy-pasted verbatim ×3 … the reviewer's drift-risk warning came true") says this hook was created to END.
Why it matters: Dead exported API plus a living fourth copy of its logic is the strongest possible evidence the shared-primitive strategy failed at this seam: behavior fixes to the hook (e.g. the F-002 tabindex bug) will not reach the copy that actually renders, and the dead primitive's 10 tests give false confidence that "the radiogroup" is covered.
What correct looks like: SettingsPanel consumes RadioGroup (or the hook), and the primitive lives; or the primitive is deleted with its tests. Half-alive is the worst state.
Confidence: HIGH — Verification: consumer sweep (only test file), SettingsPanel read, hook-user sweep (SwatchGroup + RadioGroup only).

### F-002 [S4]
Where: ui/src/components/form/RadioGroup.tsx:83-84
Code:
```
                // roving tabindex (doc 08 §4.5): only the selected option is tabbable
                tabIndex={checked ? 0 : -1}
```
What is wrong: If `value` matches no option (empty string, initial unset state, or a stale value), NO option is checked, so every input is tabIndex=-1 — the entire group has no Tab stop and is unreachable by keyboard. The roving-tabindex contract requires a fallback stop (first enabled option) when nothing is selected.
Why it matters: Any future consumer starting uncontrolled (`value=""`) ships an invisible-to-keyboard control group. Latent today only because the component has no consumers (F-001).
What correct looks like: `tabIndex={checked || (noOptionChecked && isFirstEnabled) ? 0 : -1}`.
Confidence: HIGH — Verification: read; the tab order consequence is definitional (no 0-index input ⇒ no Tab entry).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RadioOption`/`RadioGroupProps` | 6-26 | REAL | — | clean typed API |
| `RadioGroup` | 34-97 | REAL (code) / DEAD (production) | S3/S4 | F-001 dead component; F-002 tabindex hole; fieldset-disabled platform-native ✓; id derivation `${groupId}-${option.value}` breaks only on duplicate values (React key collision — caller error, unguarded) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| RadioGroup.test.tsx (10 cases) | YES for the tested behavior | the suite renders, clicks, arrow-keys, asserts roles/checked states — mutation of the hook or markup fails it |
| …but the suite's subject is dead code | — | 10 green tests + a snapshot (W15-A10 audits the .snap) verify a component no screen can reach — textbook "test keeps dead code green" |
Mutation check (reasoned): change `tabIndex` to always 0 → no test fails unless one asserts Tab stops specifically (F-002 untested); delete the component → the test file fails to import (the only tripwire). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "roving tabindex (doc 08 §4.5): only the selected option is tabbable" | l.83 | PARTIAL | true when a selection exists; no-selection ⇒ NO tabbable option (F-002) |
| "the ONE roving keydown handler for the three radiogroup-grammar primitives" (hook header) | useRadiogroupKeyboard.ts:1-4 | FALSE in spirit | a fourth implementation (SettingsPanel's PanelRadioGroup) ships beside the hook |
| "Options are native type=radio inputs … form integration from the platform" | l.29-32 | REAL | verified in markup |

## 7. Dead / duplicated / unreachable code
Dead: the entire component in production (F-001). Duplicated: PanelRadioGroup (SettingsPanel.tsx:275-330) vs this component + hook — fourth copy of the roving grammar. Unreachable in shipped product (W14) — and unreachable in dev too (no consumer).

## 8. Dependencies & flags
useRadiogroupKeyboard (shared hook — also used by SwatchGroup), form.css (`.lumen-radiogroup*`, `.lumen-choice--radio`, `--error`). No feature flags.

## 9. Open questions for the next agent
- SettingsPanel.tsx owner: can PanelRadioGroup consume RadioGroup/useRadiogroupKeyboard, or does the panel row grammar (checked prop, onSelect per row, focusRef collector) genuinely diverge? If it diverges, the shared primitive should shrink to the hook and RadioGroup should be deleted.
- SegmentedControl.tsx (my next file) and SwatchGroup: do they share the same dead-or-alive split?
