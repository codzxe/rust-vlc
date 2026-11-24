# AUDIT: ui/src/screens/player/tools/dynamics/harness.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 110 / 2 describes, 6 its |
| File verdict | REAL (executed in the committed baseline: 6/6 green; one it is env-gated and effectively a no-op in normal runs) |
| Reachable from shipped entry point? | NO (vitest suite; the harness itself is dev-only) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |

## 1. One-paragraph verdict
Two halves: (1) a dev-only vision-harness smoke test — renders the three
dynamics tools in both states (6 .dynamics-tool mounts) without crashing, plus
an env-gated standalone-HTML export for headless screenshotting; (2) four
CSS regression pins for the h14 fixes, asserting the exact calibrated rules
exist in dynamics.css (thumb-flush formula replacing the overhang bug, chevron
−7.5px calibration, tick geometry + color-mix composition, light-theme semantic
remaps with explicit NOT-matches for the old raw-hex overrides).

## 2. What actually works (verified)
- Baseline (ui-test.txt:130): `✓ src/screens/player/tools/dynamics/
  harness.test.tsx (6 tests) 115ms` — green.
- The negative assertions are real regression teeth: the old overhanging
  formula (`100% - 8px` thumb left) is asserted ABSENT (:74-76), and light-mode
  raw-hex overrides are asserted absent (:106-108).

## 3. Findings
F-001 [S4] Where: harness.test.tsx:25-32.
Code: `it("renders all three tools in frame layout without crashing", () => {
 const view = render(<DynamicsToolsHarness />);
 expect(view.container.innerHTML.length).toBeGreaterThan(0);`
What is wrong: the first half of the assertion is a tautology-grade smoke
(innerHTML non-empty); the length-6 pin carries the actual value. Acceptable
for a dev-only harness, weak as a test. Confidence: HIGH — read.
F-002 [S4] Where: harness.test.tsx:34-56.
What is wrong: the export test silently no-ops without W26C2_HARNESS_OUT
(documented and honest — "normal runs skip the export", :36), so the committed
baseline never exercises the export path incl. the CSS_FILES reads; a broken
CSS path degrades to a `/* missing */` comment silently (:40) rather than
failing. Confidence: HIGH — read.
F-003 [S4] Where: harness.test.tsx:63-109 (CSS pins).
What is wrong: pinning the stylesheet via string/regex content (the declared
states.css/tokens.css precedent) is brittle to formatting churn and proves
rule text, not rendered pixels — mitigated by the visual-matrix lane, which
owns pixel truth for these surfaces. Confidence: HIGH — read.

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| vision harness (2 its) | 24-57 | REAL | smoke + env-gated export (F-001/F-002) |
| CSS pins (4 its) | 59-110 | REAL | positive + negative regression pins (F-003 brittleness) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| renders-all-three | PARTIALLY | the 6-mount count fails; the non-empty check is tautology |
| export snapshot | NO (in normal runs) | env-gated no-op; exercises only when W26C2_HARNESS_OUT set |
| 4 CSS pins | YES | exact strings/regex incl. absence of the old rules |
## 6. Doc claims: "contract §5 comparison loop (see the lane log)" — UNVERIFIED (lane log not in tree audit scope).
## 7. Dead code: none.
## 8. Dependencies: @testing-library/react, vitest, node:fs/path (zero-dep env typing pattern).
## 9. Open questions: none.
