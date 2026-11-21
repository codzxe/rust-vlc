# AUDIT: ui/src/components/button/Toggle.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 40 / 1 (Toggle skin over SwitchCore) |
| File verdict | REAL |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (production consumers: ColorsTool.tsx rows, controls/SettingsPanel.tsx — rg `<Toggle` verified). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sed switchCore.tsx:1-90 (the core both skins share); Toggle.test.tsx survey 20-87; rg production `<Toggle` consumers |

## 1. One-paragraph verdict
A textbook thin skin: the j01-3 Lead ruling (one core, two class grammars)
implemented exactly — Toggle supplies only the `.lumen-toggle*` skin object
and forwards everything to SwitchCore, which carries the real semantics
(native button, role="switch", aria-checked, disabled-means-disabled,
localized decorative micro labels). Tests cover semantics, keyboard,
disabled, ref, labels, i18n precedence, and snapshots. No findings.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): Toggle.test.tsx
executes switch semantics (21-25), click flip (26-32), repeated activation
(33-39), keyboard operability + focusability (40-47), disabled inertness
(48-59), ref + structure (60-68), decorative labels a11y-hidden (69-77),
localized-label precedence (78-86), DOM snapshots (87+).
— verified by read: SwitchCore fires consumer `onClick` then `onChange`
with the flipped value; the skin object has all four required parts; no
logic exists in this file to drift.
— verified by rg: two production consumers (ColorsTool, SettingsPanel) +
the form family's Switch twin shares the core (j01-9 shared selector
lists in button.css per the core's docs).

## 3. Findings
None. Checked: forwarding completeness (props spread), ref forwarding,
displayName, skin-grammar completeness, i18n seam usage (in core),
disabled path, double-fire hazards (onClick+onChange documented order),
duplication (none — the point of the file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| toggleSkin | 15-20 | REAL | — | four-part grammar complete |
| Toggle | 36-38 | REAL | — | pure delegation; displayName set |

## 5. Test honesty (Toggle.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| switch semantics + checked (21-32) | YES | role/aria-checked asserted |
| repeated activation value (33-39) | YES | controlled-state roundtrip |
| keyboard operable + focusable (40-47) | YES | Space/Enter through native button |
| disabled inert (48-59) | YES | no onChange on disabled click |
| ref + structure + labels (60-77) | YES | skin parts + aria-hidden labels |
| localized labels win (78-86) | YES | i18n precedence pin (j01-4) |
Mutation check: drop role="switch" → 21 fails; fire onChange when disabled
→ 48 fails. Complete for a primitive.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "toggles → switch" ARIA contract (doc 08 §4.5) | header 23-26 | REAL | core role=switch + aria-checked; tests |
| "form/Switch is the same core in the .lumen-switch* grammar" | header 33-34 | REAL | switchCore.tsx read; j01-3/j01-9 structure verified |
| 68×32 geometry per Toggle.svg | header 26-30 | UNVERIFIED (visual) | geometry lives in button.css; no visual run in baseline |

## 7. Dead / duplicated / unreachable code
None — this file is the dedup artifact (j01-3).

## 8. Dependencies & flags
switchCore + button.css only. No feature flags.

## 9. Open questions for the next agent
- button.css (unassigned to me): verify the j01-9 claim that one shared
  selector list styles BOTH class grammars (the anti-divergence mechanism).
