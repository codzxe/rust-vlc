# AUDIT: ui/src/components/slider/Volume.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 112 / 7 (1 describe) |
| File verdict | REAL (baseline-verified green; stateful-harness + pinned-geometry pointer tests) |
| Reachable from shipped entry point? | YES — VolumeSlider is the ControlBar volume control in the player |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:131 |

## 1. Verdict
Slider contract suite with the two hard parts done right: a StatefulVolume harness that
feeds commits back as the real caller does (:8-20), and pinRect mocking getBoundingClientRect
so pointer math is tested with REAL geometry rather than skipped (:22-34). ARIA attributes,
0.01 keyboard lattice with ordered live/commit event log, Home/End clamping, left→0/right→1
pointer mapping, fill percentage, className-clobber regression, disabled inertness.
Baseline green: ui-test.txt:131 `✓ …(7 tests) 146ms`.

## 2. What actually works (verified)
- Baseline 7/7 (cite above).
- Mutation reasoning: wrong step size fails the exact order array (:60-62); pointer mapping
  inverted fails :81-83; fill regression fails :93; className clobber regression fails
  :96-101 (named past incident); disabled-but-alive fails :109-110.

## 3. Findings
None. Zero-findings list: all 7 tests read (each asserts observable ARIA/DOM/callback
evidence); the only mock is getBoundingClientRect — necessary and honestly scoped to the
geometry seam; pointer events exercised as down/move/up sequences with the commit at up
implied by the ordered-log test; no component mocks; no tautologies.

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ARIA range | YES | six attribute equalities |
| keyboard lattice | YES | exact live/commit order |
| Home/End clamp | YES | last-called values 0/1 |
| pointer scrub | YES | 10%→90% mapping |
| anatomy + fill | YES | class + --lumen-slider-fill 25 |
| className merge | YES | both classes present |
| disabled inert | YES | aria-disabled + no commit |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Simulates a real caller: feeds every commit back into value" | :7 | REAL | StatefulVolume harness |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No mocks of the component; vi.spyOn only for the rect pin.

## 8. Open questions for the next agent
None blocking.
