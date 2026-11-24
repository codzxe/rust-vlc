# AUDIT: ui/src/components/slider/Seekbar.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 161 / 1 describe, 12 its |
| File verdict | REAL (executed in the committed baseline: 12/12 green) |
| Reachable from shipped entry point? | NO (vitest suite; component is shipped) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
This is the doc-08 "truthful seek" contract pinned test-first: ARIA valuetext
formatting; honest buffer-range rendering (start/end data attributes + exact
percent styles); the duration-less guard regression (w26.k03: no Infinity-width
ranges, with a positive-duration control so the guard can't over-suppress);
scrub semantics (thumb follows the preview while the FILL stays at engine truth,
data-dragging, aria-valuenow preview, snap-back on release); commit-once/
live-during-drag event ordering; pending lock + resolve-to-engine-truth; keyboard
steps (duration/1000) + End; inert-disabled behavior with no events; custom
formatter; formatSeekTime M:SS/H:MM:SS/negative. Geometry is pinned per-element
(pinRect) — the only mock is the measurement seam.

## 2. What actually works (verified)
- Baseline (ui-test.txt:104): `✓ src/components/slider/Seekbar.test.tsx
  (12 tests) 157ms` — green.
- Read assertions cross-checked for internal consistency: 100/1000 fill=10,
  500/1000 thumb=50; 8:20 = 500s; 1:05 = 65s — all correct arithmetic.

## 3. Findings
None. Checked: fakeness (none — every assert derives from component math),
test honesty (a fill/thumb conflation, a missing pending lock, a double commit,
or a lost Infinity guard each fail a specific test), mock discipline (rect seam
only), event-order pinning (exact array equality), ARIA contract.

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| pinRect/pointer/vars helpers | 7-27 | REAL | measurement seam + CSS-var probes |
| ARIA + buffer + guard (4 its) | 30-73 | REAL | incl. the k03 regression with its positive control |
| scrub/commit/pending (4 its) | 75-123 | REAL | the §5.2.1–5.2.3 contract, exact call order |
| keyboard + inert + formatter + formatSeekTime (4 its) | 125-160 | REAL | step math + End + disabled no-events |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 12 | YES | exact strings, call-order arrays, style/CSS-var values, attribute states |
## 6. Doc claims: "doc 08 §5.2 truthful seek" behaviors — each §5.2.x clause has a matching test. REAL.
## 7. Dead code: none.
## 8. Dependencies: @testing-library/react, vitest, vi.spyOn (rect only).
## 9. Open questions: none.
