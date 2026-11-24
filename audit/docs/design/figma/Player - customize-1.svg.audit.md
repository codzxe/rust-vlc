# CLAIM AUDIT: design/figma/Player - customize-1.svg (643 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 3 (asset integrity + 2 fidelity citations) |
| REAL 3 / others 0 | |

## 1. Blunt verdict
Genuine Figma export of the customize-panel state 1: `file` -> "SVG Scalable Vector Graphics
image, ASCII text, with very long lines"; canvas 1920x1080 (sed :1 verified); 151 `<path>`
elements; one embedded `data:image/png;base64` raster used as the 1920x1080 video-frame
backdrop pattern (`pattern0_2160_6137`, sed :4, :615); dozens of `Frame N` layer ids
(w26.g08:123 confirms "Essential"/"HDR" labels live inside `Player - customize-1..4.svg`).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | design/README.md:18 "Player — customize: `Player - customize.svg`, `-1..8` — Customize panel states" | whole file | REAL | File exists (ls design/figma/); README row verified (`rg -n customize design/README.md` -> :18). |
| 2 | w26.g08:11 "DETERMINISTIC: full geometry extraction of `design/figma/Player - customize-1..4.svg`" | whole file | REAL (consumer) | File parses; 151 paths extractable; g08:123 "Essential/HDR labels inside Player - customize-1..4.svg". |
| 3 | w26.l02:19 customize dirty states "-1..-5" frames used for drag-reorder verification | whole file | REAL (frame source) | l02:19 cites `Player_-_customize-1..-5.png` renders derived from this family; the SVG is the vector source of that family. |

## 3. Contradictions
None. No doc claims the UI *matches* this sheet; w26 findings use it as reference truth
for gap claims (g08 verifies panel states 1-4 against it).

## 4. Fabrication / inflation indicators
None against this asset.
