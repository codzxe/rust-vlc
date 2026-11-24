# AUDIT: ui/src/screens/settings/areas/codecs.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 124 / 8 (2 describes) |
| File verdict | REAL (baseline-verified green; F2-gate honesty + config-never-invents are structurally enforced) |
| Reachable from shipped entry point? | YES — codecs.tsx is folded into /settings/video as the Advanced Options disclosure |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:116 |

## 1. Verdict
Pins the codecs/video fold (collapsed disclosure, codecs area appears only after click), the
cache rows bound to real schema keys, the hw-decode policy control as a CLOSED disabled
dropdown carrying the ADR-001/F2 honest reason and the shipped "Software only" default, plus
the config-never-invents rule proven by cross-checking every rendered [data-setting] against
SETTINGS_ENTRIES. Baseline green: ui-test.txt:116 `✓ …(8 tests) 299ms`.

## 2. What actually works (verified)
- Baseline 8/8 (cite above).
- Mutation reasoning: fold always-open fails :49-53 (aria-expanded false→true sequence);
  ungated control fails :74/:80-81/:87; missing gate reason fails :88-90; dead-but-alive
  (clickable disabled) fails :96-102 (click leaves no listbox, value unchanged); invented
  codec options fail :110-123 (rendered ⊆ schema keys).

## 3. Findings
None. Zero-findings list: all 8 tests read (no tautologies; asserts are DOM state, schema
constants, or set containment); mocks only at transport seam; both mount styles (routed full
SettingsScreen and bare control under SettingsProvider) exercised; the schema-truth test
reads generated constants directly (:79-82).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| codecs→video disclosure | YES | collapsed → click → area appears |
| cache rows bound | YES | data-setting selectors on real keys |
| hw policy bound + closed | YES | data-setting + haspopup + disabled |
| schema truth F2 | YES | literal featureGate/featureExposed |
| disabled with reason | YES | gate-reason node + ADR-001/F2 text |
| inert while disabled | YES | no listbox after click |
| shipped default selected | YES | defaultValue + trigger text |
| config-never-invents | YES | rendered ⊆ schema keys |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bound to the REAL schema key" | :1-3 | REAL | data-setting assertions |
| "disabled-with-reason per ADR-001" | :4 | REAL | gate-reason + text asserts |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No component mocks; generated schema constants pinned.

## 8. Open questions for the next agent
None blocking.
