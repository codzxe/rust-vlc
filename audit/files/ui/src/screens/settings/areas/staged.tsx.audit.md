# AUDIT: ui/src/screens/settings/areas/staged.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 210 / 3 (useStagedMsg, StagedRow, StagedAdvancedBar) |
| File verdict | REAL — it is the deliberate honest-dead-surface factory |
| Reachable from shipped entry point? | YES — consumed by subtitles.tsx, library.tsx, recording.tsx, privacy-network.tsx (all routed settings areas) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "StagedRow|StagedAdvancedBar"` (4 consumer areas); full read |

## 1. One-paragraph verdict
This module IS the repo's answer to dead buttons: a factory for settings rows whose backends do not exist yet — every control renders DISABLED at full visual strength with the reason in a tooltip, a visually-hidden sr-only twin (aria-describedby) for keyboard/AT users, an operable ⓘ reveal where the frame shows one, and `data-staged`/`data-staged-row` markers for tests. No control looks alive when it is not; no onChange does anything (`() => undefined` on disabled controls — correct, they can never fire). Zero findings: the pattern is honest and consistently executed.

## 2. What actually works (verified)
— verified by `rg -ln`: 4 routed settings areas compose these rows (subtitles, library, recording, privacy-network); the disabled attribute + aria-describedby wiring is present on every variant (switch L101-107, text L109, enum L111-123, path+browse L125-136, advanced bar L195-204).

## 3. Findings
None. Checked: all four control variants disabled+reasoned; no hidden enabled paths; enum's identity resolver (L119) documented against the Dropdown default.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useStagedMsg | 52 | REAL | — | seam hook via shared factory |
| StagedRow | 68-179 | REAL | — | 4 control variants, all inert+reasoned, data-staged markers |
| StagedAdvancedBar | 191-210 | REAL | — | inert disclosure bar, sr-only reason |

## 5. Test honesty
No dedicated staged.test.tsx found (exit 2 on ls); coverage rides the consumer areas' tests (recording.test etc. assert staged attributes — see recording.tsx audit). UNVERIFIED whether every consumer variant is tested; the four consumers' own test files were not all inspected (out of assignment scope except recording).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "visible, full-strength, disabled, marked `data-staged`, the honest tooltip on the row — never a dead control that looks alive, never invented rows" (L7-9) | header | REAL | code matches line-for-line |

## 7. Dead / duplicated / unreachable code
The rows this factory renders are permanently inert until schema keys land — by design, disclosed, marked. Not dead code in the deceptive sense.

## 8. Dependencies & flags
Depends on SettingInfoDot, Dropdown, Switch, TextField, Icon — all real primitives. No feature flags here (seam ids are strings supplied by consumers).

## 9. Open questions for the next agent
- How many total staged rows ship across the four consumer areas? A count would quantify the honestly-dead settings surface (recording: 10 rows per header comment; library 7; network 5/9 — census numbers in the header are per-frame, verify per-area files).
