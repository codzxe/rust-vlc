# DATA AUDIT: design/figma-renders/play.png (credit: W19-A06, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 1111, 8-bit/color RGBA, non-interlaced (md5 5355e6c0...) |
| Dimensions | 900x1111 = 9.58x render of design/figma/play.svg (94x116) |
| Cited as evidence? | YES — w26.a07.md:50/:102/:111 (icon sweep: "play.png → play/pause pills only") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The play/pause pill state family render (94x116 icon sheet at 9.58x, uniform on both
axes: 900/94 = 9.574, 1111/116 = 9.582). Pixel census matches a07.md's read:
- #D37918 (accent orange) x126,786 px — the play pill body; #6B6B6B x127,908 — the
  frame's quiet-ink elements (a07's "501-fill value" per tokens.json:718); #000000
  x700,651 canvas; #8A38F5 x24,619 = Figma annotation chrome (excluded from tokens).
- The two pill-ish bodies (orange vs #6B6B6B, 126,786 vs 127,908 px) are near-equal
  areas — consistent with a play/pause pair of the same anatomy, which is exactly
  what a07.md:111 concludes ("play.png → play/pause pills only" — i.e., no unrelated
  glyphs to harvest).
No defects. Checked: scale arithmetic both axes, pill-area census, annotation-chrome
share, 3 consumer citations.
