# AUDIT: ui/src/screens/player/tools/video/VideoSliderRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 44 / 1 (VideoSliderRow) |
| File verdict | SHALLOW (real row component; the value it renders drives local card state only — "no engine yet" by its own doc) |
| Reachable from shipped entry point? | YES — video tools (Crop/Colors/Essential/HDR) in the tools dialog + effects area |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | read; consumer trace (VideoCard passes card-local onChange); cross-check types.ts pendingCommand note |

## 1. One-paragraph verdict
A thin frozen wrapper (44 LOC) composing the shared SliderRow with the video lane's i18n seam — and the honest epitaph of the whole video-tools family sits in its own props doc: `onChange: "Live + commit sink (both drive the same local state — no engine yet)"`. The sliders drag; nothing anywhere adjusts a single pixel of video output.

## 2. What actually works (verified)
— verified by read: composition is correct (row def + value + onChange + resolveLabel); tick-domain drift fix documented (frame-measured corrections).
— verified by trace: VideoCard (L94-97) renders rows into a `hidden={!open}` body whose values are card-local useState — and that body's collapse is broken by the corrupted `.dynamics-tool__bodyidden]` rule (see DynamicsCard F-001).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/video/VideoSliderRow.tsx:32-33
Code:
```
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
``
What is wrong: the admission is honest but the shipped UX is not inert — hue/brightness/contrast/saturation/gamma/sharpen/grain/banding sliders (Colors/Essential tools) look and behave like live video adjustment. No `video.set-colors`-style command exists in the wire union (rg: only set-fullscreen/aspect-ratio/crop/zoom/delay/take-screenshot). Same dead-surface family as EQ/dynamics. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| VideoSliderRow | 37-44 | REAL (component) / SHALLOW (surface) | wrapper |

## 5. Test honesty
Video tool tests exist (CropTool.test.tsx etc.); they assert rendering + local state; cannot catch "no engine" (admitted).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no engine yet" | L32 | REAL (honest) | no video color command in the catalog |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
shared SliderRow, copy.ts seam. No env vars.

## 9. Open questions for the next agent
- Does the engine have a video-filter pipeline (hue/brightness/etc.) awaiting commands? Cross-check crates/video.
