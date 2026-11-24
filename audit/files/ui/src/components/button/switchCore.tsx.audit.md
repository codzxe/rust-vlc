# AUDIT: ui/src/components/button/switchCore.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 83 / 1 (SwitchCore component; SwitchCoreProps/SwitchSkin types) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — sole implementation behind button/Toggle.tsx and form/Switch.tsx (both consumed across settings/player UIs -> screens -> main.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln "SwitchCore"` (2 wrapper consumers); Toggle.test.tsx case scan; sed quote check |

## 1. One-paragraph verdict
Clean single-implementation core for the two switch exports: native `<button>`
(free Space/Enter), `role="switch"` + `aria-checked` (correct semantics —
checked, not pressed), controlled `onChange(!checked)`, decorative i18n
micro-labels (aria-hidden) via useSwitchLabels. No fakeness. One semantic
sharp edge: a consumer's `onClick` runs before the core's own `onChange`, and
calling `event.preventDefault()` there does NOT block the toggle.

## 2. What actually works (verified)
— Code read: disabled guard (L65) is belt-and-suspenders (native disabled
buttons do not fire click anyway); aria wiring complete (L60-61); skin classes
composed with consumer className (L62). — Toggle.test.tsx pins the contract:
"exposes switch semantics" (L21), "disabled means disabled — no change fires"
(L48), keyboard operability (L40). Not run here (no node_modules; baseline
vitest 1705 passed cited).

## 3. Findings
F-001 [S4]
Where: ui/src/components/button/switchCore.tsx:64-68
Code:
```
        onClick={(event) => {
          if (disabled) return; // disabled means disabled — never fire
          onClick?.(event);
          onChange(!checked);
        }}
```
What is wrong: consumer `onClick` cannot veto the toggle — `preventDefault()`
or `stopPropagation()` inside it does not prevent the unconditional
`onChange(!checked)` on the next line. Any host needing conditional toggling
must manage state and ignore the change instead; undocumented sharp edge.
Confidence: HIGH — Verification: read.

## 4. Per-function table
N/A (<100 LOC — compact audit). Single component; the two exported interfaces
(SwitchCoreProps, SwitchSkin) are consumed by both wrappers.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "flips state through onChange on click" (Toggle.test.tsx:26) | YES | spy assertion |
| "disabled means disabled — no change fires" (L48) | YES | no-fire assertion |
| "reports the current value back on subsequent activation" (L33) | YES | controlled round-trip |
Not run here; baseline cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a native `<button>` (Space/Enter for free) carrying `role='switch'` + `aria-checked`" | switchCore.tsx:40-42 | REAL | L57-61 |
| "ONE core implementation now backs them" (Toggle + Switch) | switchCore.tsx:6-10 | REAL | both wrappers import SwitchCore (rg) |

## 7. Dead / duplicated / unreachable code
None — the dual class-grammar (`.lumen-toggle*` / `.lumen-switch*`) is
deliberate (screens' CSS hooks), documented at L20-26.

## 8. Dependencies & flags
useSwitchLabels (i18n seam), button.css. No feature gates; presentational —
backend wiring is the host's `onChange`.

## 9. Open questions for the next agent
- form/Switch.tsx + button/Toggle.tsx wrappers (other agents if assigned):
  confirm they add nothing beyond skin classes (per the j01-3 ruling).
