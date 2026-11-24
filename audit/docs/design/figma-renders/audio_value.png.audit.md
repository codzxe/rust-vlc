# DATA AUDIT: design/figma-renders/audio_value.png (credit: W19-A08, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 275, 8-bit/color RGBA, non-interlaced (md5 b2cfadd3...) |
| Dimensions | 900x275 = 7.43x render of design/figma/audio value.svg (121x37) |
| Cited as evidence? | YES — w26.l02.md:28/:73 (chip-grammar ruling), w26.e10.md:45/:100, w26.g15.md:5 (121x37) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The standalone audio-value surface render, and the e10.md:45 distinction it exists to
prove is confirmed by census:
- e10.md:45: "The standalone `audio value.svg` (121×37 r6 `#0E0E0E` @95% + backdrop
  blur, ink `#969696`) is a different instance" — this render carries #0E0E0E x185,974
  px (the window fill) and #969696 x3,498 px (the ink) and has ZERO #090909/#8F8F8F
  pixels — the EQ-frame-mounted chip colors are absent from this asset, exactly as the
  "different instance" ruling states.
- 121x37 source dims match g15.md:5's measurement byte-exact; 7.43x uniform render
  scale on both axes.
- The W19-R7 audit of the sibling `audio value.svg` verified the 121x37 rx6 #0E0E0E
  form claim (121x37 rx6 matches form.css:553) — this PNG is that SVG's render.
No defects. Checked: scale arithmetic, the two standalone-surface colors, the
mounted-chip colors' absence (the discriminator e10 used), 4 consumer citations.
