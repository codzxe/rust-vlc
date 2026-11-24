# CLAIM AUDIT: ui/src/components/form/states.md
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Claims extracted | 14 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 11 / 1 / 0 / 0 / 0 / 1 / 1 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
Rare case: a design doc whose claims survive hostile verification. Every
mechanism claim I could execute against the tree checks out — the shared
radiogroup hook, the switch-label i18n seam, the frame-fallback tokens, the
test enforcement, the honest "pending not required" scoping. Two defects:
one STALE honest-gap bullet (a dead rule it says to delete is already
deleted), and one claim left to "verifiers" that ships contrary to frames.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Enforced by the colocated tests" | :5-6 | REAL | TextField.test.tsx 8 cases incl. :20 aria-invalid, :60 §4.4 states; all sibling components have test files (ls) |
| 2 | "min/max/step are forwarded to the native <input type=number>" | :23 | REAL | NumberField.tsx:13-16 props; j01-1/j01-11 behavior asserted in NumberField.test.tsx |
| 3 | "roving keydown handler is ONE shared hook across RadioGroup / SegmentedControl / SwatchGroup" | :45-47 | REAL | useRadiogroupKeyboard.ts imported by all three (rg: SwatchGroup:4, SegmentedControl:3, RadioGroup:3) |
| 4 | "Micro labels are aria-hidden but LOCALIZED ... useSwitchLabels() seam" | :67-68 | REAL | components/button/useSwitchLabels.ts; consumed by switchCore.tsx + Toggle.tsx (rg -l) |
| 5 | "Input tokens --color-surface-input (#2a2a2a) / --color-stroke-input (#3a3a3a) do not exist" | :118-119 | REAL | rg over tokens/generated/tokens.css -> 0 matches; form.css:66-68 ships var() fallbacks with stylelint-disable + honest comment |
| 6 | "Error color is --color-record (only red semantic token)" | :121 | REAL | tokens.css:51 --color-record: #e15032; no --color-danger exists |
| 7 | "pending/loading on form inputs: NOT required by the doc" | :106-112 | REAL | correct reading of §4.4 "where relevant"; form inputs dispatch no commands (verified across form/) |
| 8 | "NumberField steppers synthesize a ChangeEvent (no stepUp/stepDown in jsdom)" | :127-128 | REAL | documented on NumberField onChange prop (NumberField.tsx:17-18) |
| 9 | "settings.css .settings-field__bare .lumen-field__label clip rule is dead — ... can be deleted" | :140-143 | FALSE (stale) | settings.css:495 comment states the rule IS already deleted ("is DELETED — k01's j01-8") — the gap bullet lags the code |
| 10 | "ValueChip ... frames measure #969696/#8f8f8f; spec says white — ships --color-content-bright" | :85-88 | REAL-UNPROVEN | ships bright per doc; frame measurements not re-derived (design lane) |
| 11 | "Switch ... 68×32 rx6 surface track" | :60 | REAL (as spec) | geometry asserted in Switch.test.tsx (sibling); not re-measured by me |
| 12 | "hideLabel is ONE mechanism since w26.k01: every field renders aria-label ... NO label text node" | :51-53 | REAL | form.css:49 comment documents the same; snapshot tests pin DOM |
| 13 | "DOM snapshots embed React useId values — stable per file order" | :130-131 | REAL | Switch.test.tsx.snap / NumberField.test.tsx.snap (audited separately) contain :rNN: ids |
| 14 | "radio ... no separate visual skin pending a radio export in the design package" | :102-104 | UNREACHABLE (by missing design asset) | no Radio.svg in design package (per doc; not re-listed) — visual skin can never land from data |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| `.settings-field__bare` clip rule "can be deleted next time that file is touched" (:141-143) | settings.css:495 "is DELETED" | settings.css — the doc bullet is stale (claim 9) |

## 4. Fabrication / inflation indicators
None found. The doc self-reports its own gaps (missing tokens, missing
danger token, color ambiguity) instead of claiming completeness — the
opposite of inflation. The "Chromium computed-style proof" (:6) is cited
but not re-verifiable in-tree — lane-log artifact, treat as UNVERIFIED
context, not evidence.

## 5. Matrix rows
Not a matrix chunk — N/A.

## 6. Follow-ups
- Token lane: --color-surface-input/--color-stroke-input seam still open
(claim 5) — S4 debt with an honest marker.
- Update the stale gap bullet (claim 9) so the next session does not
  re-delete a deleted rule.
