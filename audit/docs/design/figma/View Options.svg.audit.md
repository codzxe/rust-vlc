# DATA AUDIT: design/figma/View Options.svg (credit: W19-A02, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: SVG Scalable Vector Graphics image, ASCII, 46 lines, 13470 B (longest line 5479 chars) |
| Dimensions | 265x104 sheet; surface rect 265x104 rx10 fill #191919 @0.95 (+blur foreignObject) |
| Cited as evidence? | YES — options.css:2/:7-9/:36/:67-69, PlaylistOptionsToolbar.tsx:38, w26.a07.md:17/:35, e09.md:52 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (space in filename) |

## Verdict
The playlist view-options menu sheet; consumer geometry claims reconcile byte-exact:
- options.css:36 "265px panels (Sorting/View Options.svg) — frame width" == sheet
  `width="265"` ✓; two rows `width="257" height="48"` ✓ = the "48px rows" claim
  (options.css:7/:67).
- Labels #8F8F8F (2 fill groups) ✓; the orange check position claim (options.css:68-69
  "check x228-242 in a 4-261 row ... 19px from the row edge") reconciles arithmetically:
  261-242 = 19 ✓ — and the render audit of View_Options.png confirmed #D37918 present.
- Row structure: 2 "Option" groups with leading "Icon" groups — matches e09.md:52's
  "2 rows (Grid View/List View) w/ leading icons" inventory.
- PlaylistOptionsToolbar.tsx:38 "View Options.svg grid mode → ThumbnailGrid" — the
  body-switch consumer; consistent with the two-row grid/list sheet.
S4: filename contains a space ("View Options.svg") — the family's recurring
shell-hostility defect. Otherwise no defects.
