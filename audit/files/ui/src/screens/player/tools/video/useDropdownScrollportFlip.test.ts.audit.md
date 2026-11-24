# AUDIT: ui/src/screens/player/tools/video/useDropdownScrollportFlip.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A03 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 193 / 12 (4 describes) |
| File verdict | REAL (baseline-verified green; pure placement math + DOM write/clear contracts) |
| Reachable from shipped entry point? | YES — useDropdownScrollportFlip.ts (299 LOC) is the video-tools dropdown placement hook (HDR tools dialog) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline ui-test.txt:95; wc source |

## 1. Verdict
Split-suite: the pure placement decision (inflow / escape-below / escape-above / capped with
two-row floor) against live-captured geometries (HDR rest states labeled with the 2026-11-14
probe), and the DOM half — applyPlacement/clearPlacement writes of markers, fixed coordinates,
cap var, and full cleanup. Baseline green: ui-test.txt:95 `✓ …12 tests 30ms`.

## 2. What actually works (verified)
- Baseline 12/12 (cite above).
- Mutation reasoning: wrong escape side fails :51-76 (exact top/left/width objects); cap
  below the two-row floor fails :90-98; leftover escape marker on inflow fails :165-173;
  incomplete clear fails :175-184; crash on missing list fails :186-192.
- jsdom's no-layout limitation is handled honestly: the header states measure() bails at zero
  geometry and the seam takes geometries as inputs (:1-8) — the tests are not pretending to
  test layout.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/video/useDropdownScrollportFlip.test.ts:109-116
Code:
    const dropdown = document.createElement("div");
    dropdown.className = "lumen-dropdown";
    const trigger = document.createElement("button");
    trigger.className = "lumen-dropdown__trigger";
What is wrong: the DOM-half tests hand-build their own tree with class names duplicated from
the real Dropdown component. If production renames `.lumen-dropdown__list`, applyPlacement's
internal querySelector would find nothing in the real app while these tests still pass against
their own copy — a decoupled-false-green window. The in-file missing-list test covers the null
path, not the drift path.
What correct looks like: mount the real shared Dropdown (the class owner) in these tests, or
import the class constants.
Confidence: HIGH — Verification: read (classes are literals here; no import of the component).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| inflow fits below | YES | exact object equality |
| inflow above-only | YES | exact object equality |
| escape below 1440×900 | YES | top 596.8 == 592.8+4 gap |
| escape above 1024×768 | YES | top 194.8 arithmetic |
| cap on roomier side | YES | side+cap exact |
| two-row floor | YES | cap 96 exact |
| escape DOM writes | YES | markers + inline styles exact |
| capped DOM writes | YES | direction marker + cap var + no inline coords |
| inflow clears escape | YES | markers null after |
| clearPlacement | YES | all markers/vars/styles gone |
| missing list | YES | no crash, no markers |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "measured against the geometry h15/g17 actually captured" | :1-8 | REAL | named probe values with dates |
| closes "h15's HDR NO" | :2 | UNVERIFIED | h15 ticket evidence outside my assignment |

## 6. Dead / duplicated / unreachable code
None (class-name duplication is F-001, not dead code).

## 7. Dependencies & flags
Plain vitest + DOM API; no React render needed — appropriate for a DOM-write seam.

## 8. Open questions for the next agent
Does the shared Dropdown component still own these exact class names? (F-001 drift check.)
