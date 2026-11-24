# AUDIT: ui/src/components/form/BrowseField.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 68 / 1 (BrowseField component) |
| File verdict | UNREACHABLE (real code, zero production mounts) |
| Reachable from shipped entry point? | NO — exported from components/form/index.ts:3-4 but no screen imports it; only its own test renders it (`rg BrowseField` outside form/ + tests -> only setting-field.css:424 comment) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "BrowseField" ui/src` (production-usage scan — empty); `sed -n '315,345p' settings/SettingField.tsx` (the real browse path); BrowseField.test.tsx case scan |

## 1. One-paragraph verdict
The component itself is well-built (label wiring, error aria, hideLabel
mechanism shared with TextField, honest disabled handling per the j01-13
note) — and nobody ships it. No screen mounts BrowseField; the settings path
rows that would want it (SettingField.tsx:328-342) hand-roll their own browse
button, which is honestly DISABLED because no file-dialog command exists in
the catalog. A tested, styled, exported component that is dead weight in the
bundle.

## 2. What actually works (verified)
— Code read: label/htmlFor wiring (L36), aria-invalid/describedby error
contract (L47-48), hideLabel aria-label mechanism (L46), native disabled
inertness comment (L52-53). — Tests (BrowseField.test.tsx, 6 cases) are
behavioral and would pass — against a component no user can reach. Not run
here (no node_modules; baseline vitest 1705 passed cited).

## 3. Findings
F-001 [S3]
Where: ui/src/components/form/BrowseField.tsx:28 (with form/index.ts:3-4 and
settings/SettingField.tsx:328-342)
Code:
```
export const BrowseField = forwardRef<HTMLInputElement, BrowseFieldProps>(
```
What is wrong: zero production consumers. The settings path rows implement
their own TextField+button combo instead (SettingField.tsx:328-342) — with the
button `disabled` and the honest "capability-unavailable" reason because the
catalog has no file-dialog command. setting-field.css:424 even styles "Any
BrowseField-hosted action inside settings rows" — CSS for a usage that does
not exist.
Why it matters: dead exported component + dead CSS hook + a test suite
validating unreachable UI; bundle and maintenance cost with no user value.
What correct looks like: either SettingField adopts BrowseField (when a
file-dialog command lands) or the component is deleted.
Confidence: HIGH — Verification: ran rg (production scan empty).

## 4. Per-function table
N/A (<100 LOC — compact audit). Single component; REAL code, UNREACHABLE verdict.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "the action button fires onAction and carries its caption" | YES | spy assertion (L20) |
| "disabled disables both the input and the action; no action fires" | YES | no-fire assertion (L27) |
| hideLabel mechanism alignment (L48) | YES | DOM shape assertion |
All honest — but they test dead code (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The action is caller-wired (`onAction`) so the component stays transport-agnostic" | BrowseField.tsx:25-26 | REAL (and moot — no callers) | rg scan |
| (SettingField) "the Browse ACTION has no file-dialog command in the catalog" | SettingField.tsx:315-317 | REAL | no dialog/pick command in commands.ts (catalog scan) |

## 7. Dead / duplicated / unreachable code
The entire component (F-001): exported, tested, styled — never mounted.

## 8. Dependencies & flags
react forwardRef/useId, form.css. No feature gates; no transport imports.

## 9. Open questions for the next agent
- The settings Browse affordance is dead pending a file-dialog command —
  check whether the desktop shell (apps/desktop files.rs) has a native dialog
  path that could be exposed as a command (tauri plugin dialog), making both
  the disabled button and this component live.
