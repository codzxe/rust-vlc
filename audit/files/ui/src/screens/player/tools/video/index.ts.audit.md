# AUDIT: ui/src/screens/player/tools/video/index.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A04) |
| Date | 2026-11-24 |
| Lines / functions | 24 / 0 (barrel: 18 exports — 10 value, 8 type) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — sole external importer ToolsDialog.tsx:39 (all four tools) → PlayerScreen.tsx:1203 ToolsDialogHost |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | rg per-symbol export-existence (all 18 exist) + external-consumer scan (file lists); rg actual import statements -> exactly one barrel importer; sed quotes verified; baseline ui-test.txt:77,123,145 |

## 1. One-paragraph verdict
All 18 exports resolve to real declarations and the four headline tools are production-mounted in the shipped tools dialog. Two real defects: (1) the header claims the barrel serves "the Settings → Video effects area" — no settings surface imports anything from this barrel (verified by exhaustive import scan); (2) 14 of 18 exports (VideoCard, VideoSliderRow, useVideoMsg, the 3 frameData symbols, 8 types) have zero external importers — the barrel's public surface is 78% dead weight.

## 2. What actually works (verified)
— verified by rg: every export exists at its source (ColorsTool.tsx:196, CropTool.tsx:146, EssentialTool.tsx:124, HdrTool.tsx:50, VideoCard.tsx:43, VideoSliderRow.tsx:37, copy.ts:23, frameData.ts:11,16,19, types.ts:25-105).
— verified by baseline ui-test.txt:77,123,145 — ColorsTool (16), HdrTool (6), CropTool (9) tests pass, all rendering through these modules.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/video/index.ts:1-3
Code: (verbatim)
```
// w26.c4 — video tools public surface (player video tools: Colors / Crop /
// Essential / HDR): the four mountable tools for the integration lane (tools
// dialog via ToolsDialogShell body) and the Settings → Video effects area.
```
What is wrong: the second integration point does not exist — rg for `from "../tools/video"` / `tools/video"` across ui/src (excluding the module itself) returns exactly ONE importer: ToolsDialog.tsx:39. The settings effects area (areas/effects.tsx) imports only from the equalizer module; the settings video area (areas/video.tsx:33-44) mounts generated schema sections and imports nothing from here. Why it matters: the comment asserts a shipped integration that is absent — exactly the doc-claims-what-code-contradicts pattern this audit hunts; a reader wiring the effects area would expect an existing seam. What correct looks like: fix the comment to name only the tools dialog, or actually mount a tool in the video effects area. Confidence: HIGH — Verification: read (exhaustive import scan).
F-002 [S3]
Where: ui/src/screens/player/tools/video/index.ts:11-24
What is wrong: 14 of 18 exports have no importer via the barrel — VideoCard, VideoSliderRow (+VideoSliderRowProps), useVideoMsg, FRAME_EXTRACTION_COLOR, FRAME_THRESHOLD_COLOR, isHexColor, and the 8 re-exported types are consumed only by deep imports inside video/ (and its tests); the barrel is the sole entry for just the 4 tools. Why it matters: dead public surface invites accidental coupling and makes the module's real API (4 tools) unreadable; same disease as variants/index.ts. What correct looks like: export the 4 tools (+useVideoMsg if shared/SliderRow ever actually imports it — today its mention there is a comment, SliderRow.tsx:27). Confidence: HIGH — Verification: read.

## 4. Per-function table
Not applicable (barrel). Export trace: 4 live (tools → ToolsDialog), 14 dead-via-barrel (F-002), all 18 declarations themselves live internally.

## 5. Test honesty
No tests in this barrel. Consumer tests (ColorsTool/HdrTool/CropTool/EssentialTool .test.tsx, baseline green) import deep paths, not the barrel — the barrel's own importability is untested.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the four mountable tools for the integration lane (tools dialog …)" | index.ts:2-3 | REAL | ToolsDialog.tsx:39 imports all four |
| "… and the Settings → Video effects area" | index.ts:3 | FALSE | no settings file imports this barrel (rg exhaustive) |
| "All state is local to each tool; controls are honestly staged" | index.ts:4-5 | REAL | verified in EqualizerTool sibling pattern; VideoCard/HdrTool carry data-staged markers (EqualizerTool.tsx:129-131 analog; see component audits) |

## 7. Dead / duplicated / unreachable code
F-002's 14 exports. Internal sharing (VideoCard/VideoSliderRow/frameData) happens via deep imports — the barrel adds nothing for them.

## 8. Dependencies & flags
No imports — pure re-export. No feature gates.

## 9. Open questions for the next agent
Was a video-effects settings embed planned and dropped (the f05 ruling embedded only the EQ)? If the embed lands, F-001's comment becomes true — until then it is a false claim in a public-surface file.
