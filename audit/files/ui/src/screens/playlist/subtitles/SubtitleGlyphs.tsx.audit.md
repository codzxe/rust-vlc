# AUDIT: ui/src/screens/playlist/subtitles/SubtitleGlyphs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 105 / 3 (SubtitleFileGlyph, MediaFileGlyph, CycleGlyph) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by MatchedPairsList.tsx, SubtitlePickRow.tsx, subtitles barrel -> Add Subtitles flow |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers (3 files incl. barrel) |

## 1. One-paragraph verdict
Three frame-ported SVG glyph components for the subtitle flow: pure
presentation, currentColor strokes, aria-hidden + focusable=false, per-glyph
frame-coordinate provenance comments. No logic to break. No findings.

## 2. What actually works (verified)
- All three glyphs consumed by production rows (MatchedPairsList, SubtitlePickRow) via the barrel.
- Aspect ratios handled: width/height derived per viewBox (e.g. `(size * 20) / 18`) — no distortion.
- Decorative correctness: aria-hidden + focusable=false on all three.

## 3. Findings
None. Checked: dead exports (none), raw colors (none — currentColor only),
a11y attributes (present), geometry sanity (viewBox matches path extents,
e.g. CycleGlyph paths span 0-13.5 within 0-14/0-18).

## 4. Per-function table (~105 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| SubtitleFileGlyph | 24-48 | REAL | page+fold+text-lines |
| MediaFileGlyph | 56-78 | REAL | filmstrip ticks |
| CycleGlyph | 84-105 | REAL | stacked arrows |

## 5. Test honesty
No dedicated tests; glyphs are covered visually via harness mounts
(SubtitleRowsHarness renders SubtitlePickRow which uses these).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pixel-perfect SVG ports" of named frames | SubtitleGlyphs.tsx:1-10 | UNVERIFIED | needs the Figma frames; internal consistency (coords cited per path) checks out |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None (React types only).

## 9. Open questions for the next agent
- None.
