# DATA AUDIT: design/figma-renders/Player_-_hide_UI.png (credit: W19-A08, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 506, 8-bit/color RGB (no alpha), non-interlaced (17972 B, md5 73c99304...) |
| Dimensions | 900x506 = 0.469x downscale of design/figma/Player - hide UI.svg (1920x1080) |
| Cited as evidence? | YES — docs/waves/w26/CONTRACT.md:49 (Hide-UI /player idle target), w26.a15.md:64 (Iter-2 harness GAPS verdict), w26.gfix-F.md:8 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The chrome-free video state frame, and it is truly chrome-free at the pixel level:
- Pixel census (PIL): 309,525 of 455,400 px (68%) are exactly #2d2b29 (45,43,41) — the
  video canvas tone — with the remainder a smooth luminance ramp (501 unique colors,
  next stops #2c2a28/#2b2927...): a video still, no UI glyphs, no top bar, no control
  strip. Consistent with the sibling SVG audit (W19-R7: 5 elements, no UI glyphs) and
  with a15.md:23's zero-chrome framing.
- 8-bit RGB without alpha: correct for an opaque video frame (frames with menu
  overlays use RGBA instead) — matches the state's meaning.
- The app side honestly declares the state UNIMPLEMENTED as of the last edit
  (PlayerScreen.tsx:355 "No hide-UI surface exists yet (Player - hide UI.svg is the
  target..."), :62/:754 comments) — so the PNG is the CONTRACT target, not claimed
  shipped; no false fidelity claim consumes it.
No defects. Checked: dims/scale, color census, alpha-mode sanity, 4 consumer citations,
app-side honesty of the target-vs-shipped distinction.
