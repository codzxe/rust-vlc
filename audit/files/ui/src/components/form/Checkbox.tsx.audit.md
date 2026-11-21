# AUDIT: ui/src/components/form/Checkbox.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 40 / 1 (Checkbox, forwardRef) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — zero production consumers. `rg -n "import.*Checkbox" src/` (excluding its own test and the form barrel): no matches. The bool settings all use `Switch` (extensions/effects/subtitles/staged areas, onboarding); the menu "checkbox" surfaces are MenuEntry `kind: "checkbox"` rendered by MenuItem's check slot (PlaylistPanel.tsx:416), not this input |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read full file + Checkbox.test.tsx (38 lines) + form.css:238-260 (the choice styles) + states.md:35-40 · `rg -n "Checkbox" ui/src` (consumer closure) · `rg -n "import.*\bCheckbox\b" src/` (zero production imports) · `sed -n '405,420p' PlaylistPanel.tsx` (the menu-checkbox that is NOT this component) · `rg -n "import.*\b(Switch\|RadioGroup)\b" src/screens src/components` (what production actually uses for bools) |

## 1. One-paragraph verdict
A correct, tiny, native-input checkbox — label htmlFor wiring, auto-id, aria-invalid + error class, disabled class + attribute, Space/checked semantics free from the platform, states.md contract matched — that nothing in the shipped app renders. The app's boolean surfaces are all Switches (five settings areas + onboarding) and its menu checkmarks are MenuItem's check slot; this primitive exists only for its own test and the form barrel. Same verdict class as BrowseField (W15-A04 file 19): real code, tree-shaken from the bundle, orphaned at source level, with the one difference that its header never claims a consumer it doesn't have.

## 2. What actually works (verified)
— The mechanics are right and test-pinned: label association via htmlFor ↔ inputId (auto-id fallback), `aria-invalid={invalid || undefined}`, the error/disabled state classes, ref forwarding (Checkbox.test.tsx:9-36).
— states.md's documented contract ("sets aria-invalid per-input") matches the implementation (:30).
— form.css carries the full choice-skin block (accent-color checked state per states.md:101, error/disabled token states) — the styles exist for a component no screen mounts.

## 3. Findings

### F-001 [S3]
Where: ui/src/components/form/Checkbox.tsx:17-38
Code:
```
export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, invalid = false, disabled, id, className = "", ...rest }, ref) => {
    const autoId = useId();
    const inputId = id ?? autoId;
```
What is wrong: the component has no production consumer. Every boolean setting renders a `Switch` (settings areas extensions/effects/subtitles/staged, onboarding's privacy toggles); every menu checkmark is MenuItem's check-slot grammar (MenuEntry `kind: "checkbox"`, e.g. PlaylistPanel.tsx:416's shuffle face). The Checkbox is imported only by its own test and the form/index.ts barrel.
Why it matters: dead source with live maintenance surface — form.css's choice block, states.md's Checkbox section, and the test all describe a component no user can reach; a future contributor "fixing" checkbox behavior here changes nothing in the app, and the drift risk with the actually-used Switch/MenuItem-check grammar grows.
What correct looks like: either delete it (component + barrel export + test + the checkbox half of the choice CSS) or adopt it somewhere the frames actually show a checkbox (none found in the audited surfaces — the frames' toggles are all Switch-styled).
Confidence: HIGH — Verification: exhaustive import grep + read of the actual bool-control consumers.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Checkbox | 17-38 | REAL (code) / UNREACHABLE (usage) | S3 | native input + label wiring correct; `type` omitted from props (forced "checkbox", :28) — good; orphaned (F-001) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Checkbox.test.tsx:9 label wiring + onChange | YES | label association + event fired |
| :18 checked readOnly renders | YES | controlled-readonly form |
| :23 invalid → error class + aria-invalid | YES | class + attribute |
| :28 disabled blocks change | YES | inert assertion |
| :35 ref + DOM snapshot | YES / WEAK | ref asserted; snapshot is structure-only |
All green: baseline ui-vitest.log.txt — but the whole suite exercises unreachable code (F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No checkbox export exists in the design package (doc 08 §1.5); styled from the accent/icon language" | header :13-15 | REAL | form.css:238+ carries the interim styles |
| states.md "Checkbox … sets `aria-invalid` per-input" | states.md:39 | REAL (code) / moot (no consumer) | :30; F-001 |
| states.md "checked: Checkbox/Radio via native `accent-color`" | states.md:101 | REAL (CSS) / moot | form.css choice block |

## 7. Dead / duplicated / unreachable code
- The entire component is unreachable production code (F-001) — barrel-exported, test-imported, screen-never-imported, tree-shaken from the build.

## 8. Dependencies & flags
- React (forwardRef/useId), form.css. No engine dependency, no feature flags.

## 9. Open questions for the next agent
- RadioGroup (states.md groups Checkbox/Radio) — is it likewise orphaned? Same grep pattern applies; if both are dead, the choice CSS block halves.
- The form barrel (form/index.ts) is accumulating dead exports (BrowseField, Checkbox) — a barrel-level audit should count exported-vs-consumed.
