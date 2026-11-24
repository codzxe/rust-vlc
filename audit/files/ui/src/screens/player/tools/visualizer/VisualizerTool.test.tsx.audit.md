# AUDIT: ui/src/screens/player/tools/visualizer/VisualizerTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 276 / 18 `it` blocks in 3 describes |
| File verdict | REAL — executed, 18/18 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:58) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
A frame-fidelity + honesty test. The "honest display contract" describe
(170-275) pins exactly the behavior that fake implementations get wrong:
resting state when no `levels` exist (never simulated bars), live bars only
when data exists, NOTHING rendered while Disabled even when levels are
supplied (204-207), empty levels array stays resting (227-233). Pixel
proportions are pinned to measured frame heights (13/36/31/41/33/21 px),
the `RESTING_LEVELS` data transcription is pinned exactly (5/14/12/16/13/8
— including the corrected bar-4=16 vs the 19 typo), and live levels are
clamped (2 → full band, -1 → 3px pill floor).

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:58 (verbatim):
```
 ✓ src/screens/player/tools/visualizer/VisualizerTool.test.tsx (18 tests) 342ms
```
- Honesty pins (lines 204-206, 227-232):
```
  it("renders NOTHING while Disabled — even with levels (frame default)", async () => {
    const { container } = render(<VisualizerTool defaultOpen levels={[0.5, 1]} />);
    expect(container.querySelector(".viz-display")).toBeNull();
```
and
```
  it("an empty levels array stays RESTING (honest: no data, no fake bars)", async () => {
```
- Exact data pin (197-202): `expect(RESTING_LEVELS.spectrum).toEqual([5, 14, 12, 16, 13, 8]);`
- Clamping pin (217-220): heights 41px (1.0), 21px (0.5), 41px (2→clamped), 3px (-1→floor).
- Keyboard model (141-159): ArrowDown moves roving focus, Enter commits `onChange("scope")`, Escape closes + refocuses trigger.
- scrollIntoView fold-reveal pinned through an instrumented prototype swap with proper restore (100-124) — including the undefined-original restore branch.

## 3. Findings
None. Zero-findings justification: all 18 assertions are exact DOM/ARIA/
data-attribute/style equalities; the mocked surface is none (component
rendered directly with props — the seam under test is the component's own
contract); failure would be loud (the `bar()` helper throws when a bar is
missing, line 19-23).

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| bar() helper | 19-23 | REAL | throws loudly on missing bar |
| expand/openSelector | 25-33 | REAL | |
| card shell describe | 35-72 | REAL | aria-expanded/hidden, staged markers, pending command id |
| mode dropdown describe | 74-168 | REAL | 5 options in order, icons count 4, selected attrs, scrollIntoView spy, onChange seam, keyboard |
| honest display describe | 170-275 | REAL | resting/live/disabled/clamped/two-tone/midline/dial |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 18 | YES | exact arrays (RESTING_LEVELS), exact px heights, exact aria-labels, negative presence checks for the disabled case |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "HONEST display contract — resting state when no levels exist (never simulated)" | header :5-7 | REAL | tested at 171-233 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
@testing-library/react, vitest, local types (RESTING_LEVELS). No gates.

## 9. Open questions
None.
