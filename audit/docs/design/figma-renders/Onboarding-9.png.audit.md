# DATA AUDIT: design/figma-renders/Onboarding-9.png (credit: W19-A07, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 627, 8-bit/color RGBA, non-interlaced (md5 cbae85d8...) |
| Dimensions | 900x627 onboarding step 4 (appearance · pink accent) |
| Cited as evidence? | YES — w26.g02.md:18 (GAPS verdict), w26.h02.md:147/:206 (rating 8.3 ALMOST, "vision 429x3, measured"), doc 08 inventory |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The pink-accent variant. g02.md:18's exact-color claim verifies by pixel census:
"#F052B3 ✓" — #F052B3 present with 3732 px (the Next CTA body), on the same step
template as Onboarding-7 (background census #090909 x370001 / #0e0e0e x121355 /
#191919 x15544 — identical modulo the accent region; button areas equal to within
2 px, 3732 vs 3734). h02.md's "vision 429x3, measured" disclosure is the honest
pattern: the rating numbers it used are DOM/PIL-measured, and the one I re-measured
(the accent fill) is exact. No defects. Checked: dims, alpha mode, accent pixel
presence + area symmetry vs Onboarding-7, background template identity, 3 consumer
citations.
