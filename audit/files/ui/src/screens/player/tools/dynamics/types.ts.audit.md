# AUDIT: ui/src/screens/player/tools/dynamics/types.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 85 / 0 (type-only module: 5 exported types) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — all 5 exports consumed by CompressorTool/SpatializerTool/StereoWidenerTool/DynamicsCard/ToolSliderRow (-> ToolsDialog -> player screen) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | per-export rg consumer scan (all consumed); catalog cross-check for the "no DSP command exists" claim |

## 1. One-paragraph verdict
Type-only mirror of video/types.ts for the audio-DSP tools. All five exports
are consumed by production files; no dead types. The staging claims are TRUE:
no compressor/spatializer/stereo-widener command exists in either catalog
(rg empty over commands.ts and command.rs) — unlike the video lane's CropTool,
this lane's justification holds. The STAGED DISPLAY RANGES contract is
documented identically (frame-calibrated defaults, engine ranges to come).

## 2. What actually works (verified)
— Consumer scan: DynamicsLabelKey -> copy.ts/index; DynamicsRowDef -> all
three tools + DynamicsCard + ToolSliderRow; DynamicsRowState -> DynamicsCard;
DynamicsToolProps -> three tools; DynamicsCardProps -> DynamicsCard.
— Claim check: `rg "set-compressor|spatializer|widener"` over
ui/src/generated/commands.ts and crates/commands/src/catalog/command.rs ->
empty. The "no such command exists yet" note (L79-80, example
"audio.set-compressor") is accurate.

## 3. Findings
None. Checked and how: dead exports (none — scan above); fakeness (type-only);
claim honesty (catalog cross-checks); reachability (consumer map). The audio
tools themselves (Compressor/Spatializer/StereoWidener) are staged local-state
surfaces by the same pattern as EssentialTool — their own audits own that
finding; nothing in THIS file is false.

## 4. Per-function table
N/A (<100 LOC — compact audit). 5 types inventoried in §2.

## 5. Test honesty
No tests (types compile-time; exercised via the three tool test files in the
same folder).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no compressor/spatializer/stereo-widener engine stage or catalog command exists yet" | types.ts:26-27 | REAL | rg empty over both catalogs |
| "No such command exists yet — the marker keeps the controls honestly staged" (audio.set-compressor example) | types.ts:79-81 | REAL | absent from both catalogs |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
i18n MessageKey + lane key files w26.c2/w26.a18 (type-only). No runtime
imports, no feature gates.

## 9. Open questions for the next agent
- CompressorTool/SpatializerTool/StereoWidenerTool (same folder): confirm
  each renders the staged markers (data-staged-tool) like the video lane, and
  that no tool dispatches anything (expected: all local-state).
