# AUDIT: ui/src/components/slider/EQSlider.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 180 / 1 describe, 10 tests, pinRect helper |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:87 "EQSlider.test.tsx (10 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg suite in baseline ui-test.txt |

## 1. One-paragraph verdict
A rigorous interaction test: ARIA slider semantics (min/max/now/valuetext/orientation/tabindex), pill anatomy with the value-chip concern kept OUT of the slider (textContent empty — a real separation-of-concerns pin), the doc-08 §5 call-order contract (live scrub events then ONE commit on release — exact array ["live:5","live:15","commit:15"]), keyboard ladder (step / Shift×10 / Home / End bounds), disabled honesty (aria-disabled + tabindex -1 + no commit + value unchanged), vertical fader geometry with real drag math snapped to the dB lattice (-17/+17), and two CSS pixel-math tests that READ the committed slider.css and prove the fill/thumb flush identity numerically at p=0/20/50/80/100 — the anti-drift technique that caught the live-measured 2.8 px overhang regression. pinRect mocks getBoundingClientRect only (jsdom has no layout engine — the sanctioned geometry seam).

## 2. What actually works (verified)
- Baseline ui-test.txt:87: `✓ src/components/slider/EQSlider.test.tsx (10 tests) 147ms`.
## 3. Findings
None. Checked: tautologies (none — exact event arrays and numeric lattices), mocks-of-subject (none — real component; getBoundingClientRect spy is layout-only), regex-over-CSS parsing is fragile-but-exact (any CSS drift fails loudly with the selector in the message).
## 4. Per-function table (test blocks)
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| ARIA over band range | 24-32 | REAL | all six attributes |
| valuetext formatting | 34-37 | REAL | "4.5 dB" |
| pill anatomy + no chip | 39-49 | REAL | track/fill/thumb + empty textContent + --lumen-slider-fill 50 |
| scrub/commit order | 51-70 | REAL | data-dragging lifecycle + exact call array |
| keyboard ladder | 72-84 | REAL | 1 → 10 → -12 → 12 |
| disabled honesty | 86-97 | REAL | inert + unfocusable + uncommitted |
| vertical ARIA | 99-106 | REAL | orientation + classes |
| vertical Y-axis scrub | 108-130 | REAL | real geometry math → lattice snap -17/+17 |
| vertical flush pixel math | 135-164 | REAL | parses committed CSS, expects the L8 travel grammar, numeric identity at 5 sample points with 1e-9 tolerance |
| horizontal inset-2 pair | 166-179 | REAL | exact CSS declarations pinned |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| scrub/commit order | YES | wrong call order fails the exact array |
| vertical scrub | YES | a sign flip (drag up = down) fails -17/+17 |
| CSS flush math | YES | the exact regression (2.8 px overhang) this pins would re-fail |
| disabled | YES | any event leak fails not.toHaveBeenCalled |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "live-measured overhang 2.8px @p=80, 4px @p=50" regression | :132-134 comment | REAL-UNPROVEN | historical measurement; the pin's existence is the proof of record |
| "the flush identity itself is pinned in Slider.test" | :174-175 | REAL-UNPROVEN | base Slider.test.tsx not in my assignment |
## 7. Dead / duplicated / unreachable code
The rule/declOf helpers are duplicated between the last two tests (S4-level duplication at most; kept as local closure pairs — not flagged as a finding).
## 8. Dependencies & flags
@testing-library/react, vi.spyOn for geometry, node:fs readFileSync of the sibling slider.css (the CSS is committed — the test reads reality, not a fixture copy). No feature flags.
## 9. Open questions for the next agent
- CSS-regex parsing will need updating if the stylesheet moves to nested selectors or CSS modules — brittle by design; acceptable trade for exactness.
