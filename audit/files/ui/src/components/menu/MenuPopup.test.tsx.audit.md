# AUDIT: ui/src/components/menu/MenuPopup.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 174 / 2 describes, 8 its |
| File verdict | REAL (executed in the committed baseline: 8/8 green) |
| Reachable from shipped entry point? | NO (vitest suite; component is shipped) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Geometry pinning for the popup viewport-clamping logic: jsdom's all-zero rects
are replaced with explicit DOMRect fixtures (documented "slider-suite pattern",
hoisted so the mount-time layout effect sees them), the window viewport is
stubbed, and the tests assert exact pixel outcomes (left/top styles for
pointer-anchored popups; translate transforms for submenus) across flip / clamp /
fit cases, including the ContextMenu wrapper path. The arithmetic in every
assertion checks out by hand (e.g. 1300−280=1020, 800−683=117; 1000−4−400=596
-> translate −641; bottom 1000->892 = dy −108). VIEWPORT_MARGIN_PX = 8 verified
in MenuPopup.tsx:29.

## 2. What actually works (verified)
- Baseline (ui-test.txt:107): `✓ src/components/menu/MenuPopup.test.tsx
  (8 tests) 204ms` — green.
- Mock discipline: only getBoundingClientRect is faked (the measurement seam);
  the component, its clamp math, ARIA roles and the ContextMenu wrapper are the
  real code under test. `afterEach` restores mocks.
- Reproduction labels cite real defect cases (w26.e02 F1 / w26.e12 F1+F2 with
  exact coordinates).

## 3. Findings
None. Checked: fakeness (no invented outputs — every expected value is derived
geometry), test honesty (mutation would move a pixel and fail the exact-string
assertions), mock-of-thing-tested (the geometry API only), leak hygiene
(restoreAllMocks), name-vs-body (all deliver the clamp behavior they promise).

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| asRect/mockMenuGeometry/stubViewport helpers | 17-43 | REAL | measurement-seam mocking only |
| renderPointerPopup | 64-78 | REAL | mounts exactly the PlayerScreen pattern |
| pointer clamping (5 its) | 80-128 | REAL | flip / clamp / fit / origin-shift / wrapper path |
| submenu clamping (3 its) | 130-170 | REAL | flip-left / shift-up / untouched |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 8 | YES | exact pixel-string assertions; any clamp-rule change moves them |
## 6. Doc claims: header comment describes the w26.f07 clamping rules — matches MenuPopup.tsx (read cross-check, margin 8). REAL.
## 7. Dead code: none.
## 8. Dependencies: @testing-library/react, vitest, vi.spyOn.
## 9. Open questions: none.
