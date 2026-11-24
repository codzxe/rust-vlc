# AUDIT: ui/src/screens/player/tools/dynamics/StereoWidenerTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 118 / 4 describe blocks, 9 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:124 "StereoWidenerTool.test.tsx (9 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg suite in baseline ui-test.txt |

## 1. One-paragraph verdict
A tight frame-pinned component test: collapsed-by-default card with a working disclosure (aria-expanded + aria-controls + hidden toggling), the four frame rows (labels, values, sliders, count), the frame's asymmetric tick pattern asserted arithmetically (24 total = 8+7+9, and the Delay Time row carries 0), staged display ranges with valuetext, real keyboard behavior (fine step 20.0→20.5 ms; Shift coarse 0.2→0.3 % with the old read-out proven GONE — a stale-render catcher), and the honest staged markers (data-staged-tool + data-pending-command + role=note text).

## 2. What actually works (verified)
- Baseline ui-test.txt:124: `✓ src/screens/player/tools/dynamics/StereoWidenerTool.test.tsx (9 tests) 312ms`.
## 3. Findings
None. Checked: tautologies (none), mocks-of-subject (none — pure component render, no transport at all), ignored tests (none), vacuous greens (none).
## 4. Per-function table (test blocks)
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| ships collapsed | 25-33 | REAL | hidden body + slider absent |
| expand/collapse round-trip | 35-47 | REAL | aria + hidden + slider absence both states |
| defaultOpen | 49-54 | REAL | Collapse naming flips |
| four frame rows | 58-67 | REAL | label+value+slider per row; slider count 4 |
| tick pattern none/8/7/9 | 69-76 | REAL | total 24 + delay-row 0 — arithmetic cross-check |
| staged ranges + valuetext | 78-85 | REAL | min/max/valuetext per frame |
| keyboard fine step | 89-96 | REAL | read-out + valuetext update together |
| shift coarse step | 98-106 | REAL | old text proven absent (stale-render catcher) |
| staged markers | 110-117 | REAL | honest pending-command seam named |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all nine | YES | concrete aria attributes, counts, exact value strings; the tick arithmetic fails on any per-row drift |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| frame "Audio - Stereo Widener.svg" fidelity | header :1-5 | REAL-UNPROVEN | frame SVG not re-extracted here; the value pins are the committed record |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
@testing-library/react only. No feature flags.
## 9. Open questions for the next agent
- The pending command "audio.set-stereo-widener" — when the engine command lands, this staged marker + note must flip to a live-binding test.
