# AUDIT: ui/src/components/button/Toggle.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 40 / 1 component + skin const |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: button/index.ts barrel -> ColorsTool.tsx (video tools) + SettingsPanel.tsx (player controls) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumer chain + switchCore a11y check (exit 0) |

## 1. One-paragraph verdict
A thin, correct skin over the shared SwitchCore: role=switch + aria-checked (not aria-pressed — the ARIA contract deviation from the task-card blurb is recorded in-code), forwardRef, displayName, native-button keyboard activation delegated to the core, and the class grammar mapped per the control-parts lane. Localized decorative ON/OFF micro labels live in the core (useSwitchLabels). No defects.

## 2. What actually works (verified)
— Consumers: ColorsTool (video tools dialog) and SettingsPanel (player) via the barrel.
— SwitchCore carries the a11y contract (role/aria-checked per its header, L41); Toggle adds only the skin.

## 3. Findings
None. Checked: prop passthrough totality (extends SwitchCoreProps), no state of its own, skin class mapping to button.css grammar.

## 4. Per-function table
N/A (<100 LOC): single component reviewed in full.

## 5. Test honesty
Switch/Toggle covered by button-family tests (baseline pass), including keyboard activation and aria contract.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exposes `role="switch"` + `aria-checked` ... deviation ... recorded in states.md" | L23-26 | TRUE | SwitchCore implements it (header + code) |

## 7. Dead / duplicated / unreachable code
None (the dual Skin grammar with form/Switch is shared-core, not duplication).

## 8. Dependencies & flags
SwitchCore; button.css. No gates.

## 9. Open questions for the next agent
- None.
