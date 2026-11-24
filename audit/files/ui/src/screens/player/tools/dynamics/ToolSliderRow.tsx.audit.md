# AUDIT: ui/src/screens/player/tools/dynamics/ToolSliderRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 44 / 1 component |
| File verdict | REAL (thin lane wrapper over the shared SliderRow; the dynamics family it serves is honestly STAGED — local state only, no engine commands exist) |
| Reachable from shipped entry point? | YES — rendered by CompressorTool/SpatializerTool/StereoWidenerTool via DynamicsCard → tools dialog → player. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; rg staging markers in DynamicsCard.tsx (:91-92 data-staged-tool/data-pending-command, :138 role="note" staged line); rg audio compressor/spatializer commands in generated union (ABSENT — audio.* is volume/mute/devices/delay only) |

## 1. One-paragraph verdict
Correct consolidation wrapper: the row grammar lives once in tools/shared/SliderRow (the documented 1:1-copy drift fix), this file only adds the lane's i18n resolver. The family context is the honest-staged pattern: DynamicsCard carries data-staged-tool + data-pending-command + a visible role="note" staged line, and the wire union has no compressor/spatializer/widener commands — like the visualizer, these audio tools are interactive local state with zero backend, visibly marked. The row's own doc admits it: "both drive the same local state — no engine yet."

## 2. What actually works (verified)
— rg: staged markers real (DynamicsCard.tsx:91-92,138); shared SliderRow exists (tools/shared); dynamics suites PASS in baseline (dynamics/harness.test.tsx seen in baseline listing).

## 3. Findings
F-001 [S4] ToolSliderRow.tsx:32 — `onChange` doc says "no engine yet"; confirmed: no dynamics commands in the wire union. Recorded gap, not a bug — same family as VisualizerTool's staged marker. Confidence: HIGH — rg union.

## 4. Per-function table
n/a (<100 LOC).

## 5. Test honesty
Compressor/Spatializer/StereoWidener suites (baseline PASS) exercise these rows.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 1:1 copy that had already drifted … is gone as soon as it migrates (k11)" | ToolSliderRow.tsx:11-13 | TRUE for dynamics; video migration noted as pending | shared/SliderRow exists; VideoSliderRow not in my files — UNVERIFIED whether migrated |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
shared SliderRow, lane copy. No flags.

## 9. Open questions for the next agent
- Did VideoSliderRow migrate to the shared row (k11) as promised? If not, the duplication the comment says is "gone" persists one lane over.
