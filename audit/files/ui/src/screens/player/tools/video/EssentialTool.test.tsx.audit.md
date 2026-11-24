# AUDIT: ui/src/screens/player/tools/video/EssentialTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 171 / 10 it() tests, 4 describe blocks |
| File verdict | REAL (10/10 pass in baseline; pixel-math regression pin included) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 105 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Honest frame-true test of the Essential video card: collapsed-by-default + aria-controls
wiring, the 8 label-only slider rows pinned in ORDER with per-row degree-tick counts, an
actual pixel-math regression pin (the 2px-inset fill domain: gamma tick 1 at
`left: calc(2px + 0.1 * (100% - 4px));` vs the pre-fix raw-percent drift — with jsdom's
calc re-serialization honestly documented), staged display ranges + aria-valuetext for
every row, keyboard commit + End-clamp, and the honest staging markers
(`data-pending-command="video.set-essential"` + staged note — the tool is an UNBOUND seam
and the test pins that disclosure).

## 2. What actually works (verified)
- Baseline: ui-test.txt line 105 `✓ .../EssentialTool.test.tsx (10 tests) 378ms`; count 10 == 10.
- Read-verified strongest rows: `expect(gammaTicks[0]?.getAttribute("style")).toBe("left: calc(2px + 0.1 * (100% - 4px));");`
  plus the negative pin `expect(...).not.toMatch(/^left: [\d.]+%;$/);` — the pre-fix paint
  would fail BOTH.
- Clamp: `fireEvent.keyDown(grain, { key: "End" });` -> `aria-valuetext` "100 %".

## 3. Findings
F-001 [S4] Lines 15-24 `FRAME_ROWS` and lines 60-70 `byId` duplicate the same 8-row
label→tick data in two shapes; a row change must be edited twice (drift hazard inside one
file). Confidence: HIGH — read.
F-002 [S4] The style assertions depend on jsdom's calc() canonical serialization (the
comment itself says so) — a jsdom behavior change, not a product change, trips the test.
Disclosed in-line. Confidence: HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| FRAME_ROWS | 15-24 | REAL | S4 duplication with byId |
| describe card shell | 26-43 | REAL | collapsed/defaultOpen |
| describe frame rows | 45-128 | REAL | order, ticks, calc regression, valuetext, label-only |
| describe local state | 130-145 | REAL | commit + clamp |
| describe honest staging | 147-171 | REAL | staged markers + note |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| paints degree ticks on the 2px-inset fill domain | YES | exact style strings + negative raw-percent pin |
| carries the frame's per-row degree-tick counts | YES | per-row count equality |
| marks the card staged with the pending command marker | YES | attribute + note content |
| clamps at the range maximum | YES | End -> 100 % |

## 6. Doc claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the sliders' aria-valuetext still reports every value" (header lines 4-5) | file itself | REAL | valuetext pins for 4 distinct rows |
| 10 tests pass | baseline ui-test.txt:105 | REAL | vitest line |

## 7-9. Dead code / deps / open questions
None / vitest only / is `video.set-essential` served by the CLI host? (same staged-seam
question as audio.set-eq — W-next.)
