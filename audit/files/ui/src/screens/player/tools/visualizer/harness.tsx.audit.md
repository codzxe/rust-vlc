# AUDIT: ui/src/screens/player/tools/visualizer/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 90 / 3 (HarnessCell, VisualizerSelectorStates, VisualizerToolStates) |
| File verdict | REAL (dev-only harness, honestly labeled) |
| Reachable from shipped entry point? | NO — by design (rg: zero imports outside harness.test.tsx; not exported from visualizer/index.ts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg harness imports (none outside its test); sed quote verification |

## 1. One-paragraph verdict
Dev-only vision harness exactly as labeled: composes the visualizer selector's
two frame states and the tool card's product states for screenshot comparison
against the Figma render. Not imported by any app route (verified). The
telling detail it exposes: the ONLY mount in the entire tree that passes
`levels` (the live-data path) is this harness line 84 — hard-coded demo
numbers — confirming the visualizer's live path is unreachable in the product
(my types.ts finding F-001).

## 2. What actually works (verified)
— rg: no app imports of harness (exit 1 on the no-match search for non-test
consumers) — the "never imported by app routes" claim holds.
— HarnessCell caption pass-through pattern keeps captions out of the
frame-exact cell (22-34) — composition discipline documented.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/visualizer/harness.tsx:81-85
Code:
```
        <VisualizerTool
          defaultOpen
          defaultMode="spectrum"
          levels={[0.2, 0.55, 0.35, 0.9, 0.5, 0.72, 0.4, 0.65, 0.3, 0.84, 0.45, 0.6]}
        />
```
What is wrong: Nothing functionally — but this is the ONLY levels-fed mount in
the repo. The "live" visualizer rendering exists solely in a dev harness with
hand-picked constants; no production code path can produce it. Corroborates
the dead-seam finding rather than adding a new defect.
Confidence: HIGH — Verification: read + rg levels= across tree.

## 4. Per-function table
Not applicable (<100 LOC; 3 presentational components, all REAL).

## 5. Test honesty
harness.test.tsx renders these to static HTML for screenshots — presence-only
verification of frame composition; owned by another agent.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" | harness.tsx:1-2 | REAL | rg: no non-test consumers |

## 7. Dead / duplicated / unreachable code
The file itself is intentionally unreachable from production — that is its
contract. No duplication issues.

## 8. Dependencies & flags
VisualizerModeSelect, VisualizerTool, useVisualizerMsg, harness.css. No flags.

## 9. Open questions for the next agent
None.
