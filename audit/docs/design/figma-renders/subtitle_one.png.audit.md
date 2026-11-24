# DATA AUDIT: design/figma-renders/subtitle_one.png (credit: W19-A06, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 414, 8-bit/color RGBA, non-interlaced (md5 a1864c65...) |
| Dimensions | 900x414 = 1.733x render of design/figma/subtitle one.svg (519x239) |
| Cited as evidence? | YES — ui/src/screens/playlist/subtitles/harness.css:26-28 ("subtitle one/multiple.png = 519px × 1.73"), harness.tsx:4, AddSubtitlesDialog.tsx:12, w26.b11.md:6 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (space in SVG filename) |

## Verdict
The single-subtitle pick frame of the Add-Subtitles dialog family, and the consumer's
scale arithmetic reconciles exactly:
- harness.css:28 `zoom: 1.73; /* frame: subtitle_one.png 900/519 */` — 900/519 =
  1.734 ✓ and the PNG itself is 519x239 x 1.733 = 900x414 on both axes ✓ (uniform
  scale, no distortion).
- Consumers chain consistently: harness.tsx:4 names {subtitle_one, subtitle_multiple,
  sub_filename}.png as the state frames; AddSubtitlesDialog.test.tsx:2 asserts against
  those bodies' chrome — the asset is the real comparison oracle for that test family.
- Sibling SVG `subtitle one.svg` exists at design/figma/ (519x239) — same space-in-
  filename hygiene defect as the family (S4, recurring).
No content defects. Checked: render-scale arithmetic both axes, SVG source presence,
4 consumer citations, dialog-family naming consistency.
