# AUDIT: ui/src/screens/player/tools/visualizer/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 54 / 0 (stylesheet, 6 selectors) |
| File verdict | REAL (dev-only, as declared) |
| Reachable from shipped entry point? | NO — by design: imported ONLY by harness.tsx (L17), which its header declares "DEV-ONLY vision harness (never imported by app routes)"; not referenced by App/main routes (verified by scan) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "harness.css" harness.tsx` (only import); `rg -ln "w26c3-"` (only harness.tsx); route scan (App.tsx/main.tsx) |

## 1. One-paragraph verdict
Exactly what it claims: a dev-only vision-harness stylesheet reproducing the
"Audio Visualizer.svg" frame geometry (406px stage, 22px gutters, 362px
selector, 360px tool cards, 20px state gaps). All six `.w26c3-*` selectors are
consumed by harness.tsx and by nothing else; the file never loads in app
routes. Honest tooling, zero user-facing surface.

## 2. What actually works (verified)
— Import chain: harness.css <- harness.tsx:17 only. `.w26c3-*` classes appear
only in harness.tsx (rg). — App routes never import the harness (scan of
App.tsx route table / main.tsx). — Geometry values are internally consistent
(362 + 2×22 = 406 stage width).

## 3. Findings
None. Checked and how: shipped-surface leakage (none — route scan); dead
selectors (none — all consumed by the harness); token discipline (semantic
tokens used; raw px documented as frame measurements).

## 4. Per-function table
N/A (54 LOC stylesheet; 6 selectors: w26c3-stage/caption/cell/frame-open +
two child combinators — all consumed).

## 5. Test honesty
harness.test.tsx exists in the folder (asserts the harness composition incl.
this css file in its snapshot list — L25 pattern seen in options/harness.test);
dev tooling tested as tooling. Not run here (no node_modules; baseline cited).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness styles (harness.tsx; never imported by app routes)" | harness.css:1-2 | REAL | import + route scans (§2) |

## 7. Dead / duplicated / unreachable code
None (the whole file is intentionally unreachable from shipped routes — that
is its contract, not a defect).

## 8. Dependencies & flags
Semantic tokens + frame-measured px. No imports; no feature gates.

## 9. Open questions for the next agent
- None for this file. (See my VisualizerDisplay.tsx audit for the real
  visualizer findings.)
