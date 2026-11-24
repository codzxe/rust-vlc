# AUDIT: ui/src/screens/player/tools/visualizer/types.ts
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 140 / 2 (restingLevels + 3 exported data/type blocks) |
| File verdict | REAL (types for an UNWIRED display) |
| Reachable from shipped entry point? | YES (VisualizerTool.tsx:29 imports; mounted at ToolsDialog.tsx:130 and effects.tsx:130 — both render `<VisualizerTool />` with NO levels/onChange) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg VisualizerTool mounts (both pass no props); sed quote verification |

## 1. One-paragraph verdict
Clean type/data module whose own header confesses the master-question answer
for the visualizer: "no FFT/levels source exists yet, so absent levels render
the frame's RESTING depiction ... never animated, never simulated". The five
mode ids, resting bar heights (frame-measured, with a corrected transcription
defect documented at 85-87), and props contract are real; the wire seam
(`levels`, `onChange`) exists but NO production mount passes either — both
ToolsDialog.tsx:130 and effects.tsx:130 render the bare `<VisualizerTool />`,
so the display is a static picture forever. Honest by construction, dead by
wiring.

## 2. What actually works (verified)
— rg: exactly two production mounts, both prop-less — the "honest rendering"
path (resting state) is the only path that ever executes.
— restingLevels (98-102): normalizes raw frame heights by max — correct math,
documented 19→16 correction (85-87).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/tools/visualizer/types.ts:13-17, 125-128 + mounts ToolsDialog.tsx:130 / effects.tsx:130
Code:
```
 * display — renders the selected mode from a `levels` array (one entry
 *   per bar, normalized 0..1). HONEST RENDERING: no FFT/levels source
 *   exists yet, so absent levels render the frame's RESTING depiction of
```
What is wrong: The visualizer is user-selectable across 4 bar modes
(spectrometer/scope/spectrum/waveform) in a shipped dialog, but with no levels
source and no host passing levels, every mode renders the same kind of static
art. The mode dropdown is a live control whose only effect is swapping which
static picture shows. Dead interactive surface in the tools dialog (the
equalizer's sibling — the header at equalizer.css:44-45 names
"dynamics/visualizer/video" as the staged family).
Confidence: HIGH — Verification: read + rg mount props.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VisualizerLabelKey/Mode/ModeDef types | 26-47 | REAL | - | type-only |
| VISUALIZER_MODES | 50-75 | REAL | - | frame order, icons wired |
| RESTING_LEVELS + restingLevels | 89-102 | REAL | - | documented frame transcription fix |
| VisualizerToolProps | 105-140 | REAL | F-001 context | levels/onChange seams never passed in production |

## 5. Test honesty
No tests in this file; visualizer tests owned elsewhere.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never animated, never simulated — no idle shimmer" | types.ts:15-17 | REAL | no timers/animation in the display's no-data path (per types; VisualizerDisplay owned by another agent) |

## 7. Dead / duplicated / unreachable code
`levels`/`onChange`/`defaultOpen`/`defaultMode` props are wire-ready seams
with zero production callers — dead parameters by the checklist definition,
intentional (documented host contract).

## 8. Dependencies & flags
i18n key types (w26.c3, w26.a18), ModeIcons. No flags.

## 9. Open questions for the next agent
VisualizerDisplay.tsx owner: verify the spectrometer dial's `live ? levels :
undefined` path (VisualizerDisplay.tsx:86) is truly unreachable in production
mounts (consistent with F-001).
