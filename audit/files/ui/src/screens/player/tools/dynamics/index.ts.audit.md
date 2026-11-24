# AUDIT: ui/src/screens/player/tools/dynamics/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 17 / 0 (barrel module: 6 value exports + 6 type exports) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by player/dialogs/ToolsDialog.tsx (imports the three tools; mounts `<CompressorTool/> <SpatializerTool/> <StereoWidenerTool/>` at L127-130) -> player screen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | consumer scan (`rg -ln "tools/dynamics"` -> ToolsDialog.tsx); export-vs-usage cross-check |

## 1. One-paragraph verdict
A clean barrel. The header is honest about what it exports: three mountable
tool cards whose "state is local to each card; controls are honestly staged"
(the staged pattern audited at types.ts level in this run). External consumer
is ToolsDialog; the Settings -> Audio effects area also mounts them per the
header (effects.tsx was seen importing the eq seam family).

## 2. What actually works (verified)
— ToolsDialog.tsx imports from this barrel and mounts all three tools
(rg + the ToolsDialog.tsx:127-130 read from my EssentialTool audit). — All
exported names resolve to files in the folder (ls cross-check during the
types.ts audit). — DynamicsCard/ToolSliderRow exports are the shared grammar
also used by the video lane's VideoSliderRow wrapper (documented in
video/types.ts header).

## 3. Findings
None. Checked and how: dead exports (ToolSliderRowProps/DynamicsLabelKey etc.
consumed at least intra-lane + by siblings — see types.ts audit consumer
scan); misleading claims (none — the staged nature is stated in the header);
reachability (ToolsDialog verified).

## 4. Per-function table
N/A (<100 LOC barrel).

## 5. Test honesty
Exercised via the three tool test files (Compressor/Spatializer/StereoWidener
suites exist in the folder — other agents' scope).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "All state is local to each card; controls are honestly staged (see DynamicsCard)" | index.ts:3-4 | REAL-UNPROVEN | consistent with types.ts staged contract (audited); the three tool bodies are other agents' files — expected same local-state pattern as EssentialTool |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Re-exports only; no feature gates.

## 9. Open questions for the next agent
- Compressor/Spatializer/StereoWidener tool bodies: confirm each renders the
  data-staged-tool marker (DynamicsCard) — completing the staged-surface
  census of the Tools dialog (VIDEO tab confirmed staged; AUDIO tab expected
  staged; visualizer staged).
