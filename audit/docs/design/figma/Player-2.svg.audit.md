# CLAIM AUDIT: design/figma/Player-2.svg (606 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 2 (asset integrity + fidelity citation) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine Figma export of a player-screen variant: `file` -> "SVG Scalable Vector Graphics
image, ASCII text, with very long lines"; canvas 1920x1080 (sed :1); root group id
"Player" with pattern raster backdrop (`pattern0_970_3792`, sed :4); 93 `<path>` elements;
18 embedded `data:image/png;base64` rasters (rg census) — this variant is raster-heavy
(thumbnail/list browse views), consistent with w26.l01:94 citing Player-2's
"list/thumb browse view (32×32 r2 thumbs)".

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Player family variant consumed as frame evidence | whole file | REAL | w26.g03:9 "(`design/figma/Player.svg`, `Player-1.svg`, `Player-2.svg`, `Player-3..6.svg`, ...`)" — DOM parsing source list; w26.l01:94 cites Player-2's browse view. |
| 2 | Raster payload is real PNG data | :4+defs | REAL | 18x `data:image/png;base64` (rg -o census); pattern fill referenced at :4. |

## 3. Contradictions
None. w26.l01:94 states this variant "has no reachable state" in the shipped UI — a gap
claim about the UI, using this SVG as truth; consistent, not contradicted.

## 4. Fabrication / inflation indicators
None against this asset.
