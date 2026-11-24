# DATA AUDIT: design/figma-renders/setting_category.png (credit: W19-A04, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 553, 8-bit/color RGBA, non-interlaced (md5 60ca43e8...) |
| Dimensions | 900x553 = 3.95x render of design/figma/setting category.svg (228x140) |
| Cited as evidence? | YES — w26.l04.md:21 (MATCH verdict), w26.i12.md:61, w26.gfix-B.md:57, w26.g20.md:7, CONTRACT.md:57 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The settings-category menu frame; the live-measured MATCH claims of l04.md:21
re-verify by pixel census:
- "active row bg **#222222 rx2 h40** + label **#EAEAEA** (computed rgb(234,234,234))
  + accent icon" — #222222 present (110,411 px — the h40 row band at 3.95x), #EAEAEA
  present (1,986 px — the label glyphs), #D37918 present (2,398 px — the accent icon)
  ✓ all three.
- Annotation chrome #8A38F5 x6,629 px = export frame, excluded from tokens per
  tokens.json header.
- l04.md:21's "measured live" claim was against the app's /settings/video sidebar —
  this frame supplies the reference values, which is exactly what the census confirms.
No defects. Checked: scale arithmetic both axes, the 3 claimed colors by census,
annotation-chrome share, 5 consumer citations.
