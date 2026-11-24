# CLAIM AUDIT: design/figma/file context menu.svg (binary/vector asset)
| Field | Value |
|---|---|
| Auditor | W19-A06 (recovered by W19-R4) |
| Claims extracted | 3 (integrity + dimensions + fidelity-evidence use) |
| REAL 3 / others 0 | |

## 1. Blunt verdict
Genuine Figma-export SVG: `file` -> "SVG Scalable Vector Graphics image, ASCII
text"; root `width="265" height="584"` (python3 re-verified), 83 ids, HTML
foreignObject frame content, 134 lines. PNG render twin
`design/figma-renders/file_context_menu.png` present.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 12-row file context menu frame, 265×584 | whole file | REAL | `docs/waves/w26/analysis/menus-control-parts.md:87` "## A3. `file context menu` (265×584)"; w26.h16.md:27 "12-row file context menu ... verified below". |
| 2 | Used as fidelity oracle for the media-list context menu | whole file | REAL | w26.h16.md:137,187 grade impl panel 280px against "file context menu.svg's" 584-height geometry; w26.h18.md:110 compares icon accent painting. |
| 3 | Carries 1-4 #AB6314 icon elements per row | w26.i16.md:64 | REAL | `rg -o '#AB6314' design/figma/'file context menu.svg' \| wc -l` -> 17 (16 lines) across ~12 row icons — consistent with "1-4 per row"; i16.md:64 cites exactly this file. |

## 3. Contradictions / 4. Fabrication indicators
None. Asset is design-source data, not a behavior claim; every consumer uses it
as the measuring stick, and the impl is graded AGAINST it (h16/h18 record
defects vs this frame — oracle use, not evidence laundering).
