# AUDIT: ui/src/screens/player/tools/video/types.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 105 / 0 (type-only module: 7 exported types) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — every export consumed by the four video tools + VideoCard/VideoSliderRow -> ToolsDialog -> player screen (per-export consumer scan below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | per-export `rg -l "\b<Type>\b" ui/src` scan (all 7 consumed); catalog cross-check for the "no command exists" claims |

## 1. One-paragraph verdict
Pure type definitions, no runtime code, no dead exports: all seven types
(VideoLabelKey, VideoToolProps, VideoCardProps, HdrToolProps,
VideoSliderRowDef, VideoSliderRowState, CropAxis) are imported by production
files. The doc-comments are the interesting part: they define the STAGED
DISPLAY RANGES contract ("ranges calibrated to land the frame's thumb
position — the frames are the design authority; engine ranges replace these
when the video effects command layer lands") — an honest admission that the
slider ranges are design props, not engine data. The "no video-effects
command exists" claim is TRUE for colors/essential (catalog has none);
CropTool's pendingCommand choice (`video.set-crop`) contradicts this file's
guidance (that command DOES exist — see my CropTool audit F-001).

## 2. What actually works (verified)
— Consumer scan (rg per export): VideoLabelKey -> ColorsTool/HdrTool/copy;
  VideoToolProps -> EssentialTool/ColorsTool/CropTool; VideoCardProps ->
  VideoCard; HdrToolProps -> HdrTool; VideoSliderRowDef -> EssentialTool/
  ColorsTool/VideoSliderRow; VideoSliderRowState -> EssentialTool;
  CropAxis -> CropTool (the CROP_PAIRS typing — no longer dead, w26.k11).
— Claim check: `video.set-colors`/essential-effects commands absent from
  ui/src/generated/commands.ts and crates/commands/src/catalog/command.rs
  (verified in my EssentialTool audit — rg empty). The types.ts example
  claim is TRUE; EssentialTool.tsx:22's "types.ts note" cross-reference is
  accurate.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/video/types.ts:70-74
Code:
```
 * Ranges are STAGED DISPLAY RANGES: no video-effects engine stage or catalog
 * command exists yet (the effects-area staged pattern), so each range is
 * calibrated to land its default value at the frame's SVG-measured thumb
```
What is wrong: nothing false — but the blanket wording covers CropTool too,
whose awaited command (`video.set-crop`) DOES exist in the catalog; the
file-level guidance invites the false premise audited in CropTool.tsx F-001.
Doc precision debt only.
Confidence: HIGH — Verification: read + rg (catalog).

Zero S0-S3 findings. Checked and how: dead exports (none — all 7 consumed,
scan above); fakeness (type-only, nothing to fake); claim honesty (catalog
cross-checks); reachability (consumer map).

## 4. Per-function table
N/A — type-only module; the 7 types are inventoried in §2 with consumers.

## 5. Test honesty
No tests (types are compile-time; exercised via the four tool test files).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no video-effects engine stage or catalog command exists yet" (for slider rows) | types.ts:70-71 | REAL for colors/essential | rg empty over both catalogs (EssentialTool audit) |
| "e.g. 'video.set-colors'. No such command exists yet" | types.ts:51-53 | REAL | absent from commands.ts + command.rs |
| "the export was dead — the map now types through it" (CropAxis) | types.ts:102-104 | REAL | CropTool.tsx:48 CROP_PAIRS uses CropAxis |

## 7. Dead / duplicated / unreachable code
None (post-w26.k11; the formerly-dead CropAxis now consumed).

## 8. Dependencies & flags
i18n MessageKey + lane key files w26.c4/w26.a18 (type-only imports). No
feature gates, no runtime imports.

## 9. Open questions for the next agent
- ColorsTool.tsx + HdrTool.tsx inherit this staged contract — verify their
  pendingCommand strings against the catalog individually (the set-crop
  lesson: each claim needs its own check).
