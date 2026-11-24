# CLAIM AUDIT: design/figma/Settings - network.svg (381 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 2 (integrity + fidelity citation) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine Figma export of the Settings > Network screen: `file` -> SVG ASCII; canvas 1048x865
(sed :1); 93 `<path>` elements; ZERO base64 rasters (pure vector). Same sheet family as the
other Settings frames (backdrop-blur foreignObject + clip0_6543_15649).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Frame consumed as network-settings oracle | whole file | REAL | Consumers: docs/waves/w26/ratings/w26.h11.md, docs/waves/w26/analysis/tools-settings.md (rg -ln "Settings - network" docs/ design/). |
| 2 | Vector-only frame (no raster payload) | whole file | REAL | `rg -c base64` -> 0; 93 paths (census). |

## 3. Contradictions / 4. Fabrication indicators
None.
