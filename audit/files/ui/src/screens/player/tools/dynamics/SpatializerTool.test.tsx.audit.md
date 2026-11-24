# AUDIT: ui/src/screens/player/tools/dynamics/SpatializerTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 121 / 4 describes, 9 its |
| File verdict | REAL (executed in the committed baseline: 9/9 green) |
| Reachable from shipped entry point? | NO (vitest suite; component is shipped) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Frame-faithful pinning of the spatializer card: collapsed-by-default with a
wired expand toggle (aria-expanded + aria-controls + hidden body + sliders
absent/present), defaultOpen control, the five frame rows with label/value/
slider shape and exact default values (8.0/10.0/4.0/5.0x2), the data-driven
tick counts (46 = 10 + 4x9, with 4 covered-by-thumb — pinned, not eyeballed),
ARIA valuetext format, local-state keyboard interaction (ArrowRight 5.0 -> 5.5,
Home clamp to 0.0), and the honest staged markers (data-staged-tool
spatializer + data-pending-command audio.set-spatializer + a "staged" note).
No mocks at all.

## 2. What actually works (verified)
- Baseline (ui-test.txt:128): `✓ src/screens/player/tools/dynamics/
  SpatializerTool.test.tsx (9 tests) 346ms` — green.
- Tick-count arithmetic consistent (46 = 10+36; 4-of-5 covered with the
  Width-at-max exception documented in-test :77-78).

## 3. Findings
None. Checked: fakeness (none), test honesty (a lost collapse, a wrong
default, a broken clamp, or a dropped staged marker each fail a named test),
value collision handling (two 5.0 defaults asserted via getAllByText length 2
— the multi-instance case is handled honestly, unlike naive getByText which
would throw).

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| card shell (3 its) | 25-56 | REAL | collapse/expand/defaultOpen ARIA |
| frame rows (3 its) | 58-92 | REAL | rows + values + 46-tick pin + valuetext |
| local slider state (2 its) | 94-110 | REAL | keyboard commit + clamp |
| honest staging (1 it) | 112-121 | REAL | staged markers + note |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 9 | YES | exact values/counts/attributes; interaction state changes |
## 6. Doc claims: "pins the spatializer card against its frame" — every frame element asserted. REAL.
## 7. Dead code: none.
## 8. Dependencies: @testing-library/react, vitest.
## 9. Open questions: none.
