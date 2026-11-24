# CLAIM AUDIT: docs/waves/w26/analysis/menus-control-parts.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 6 (method + measurement claims, sampled from a 641-line anatomy spec) |
| REAL 5 / REAL-UNPROVEN 1 / FALSE 0 | |

## 1. Blunt verdict
An implementation-lane analysis doc (w26.a19) that treats the Figma SVG XML as parseable ground truth and derives cap-heights, offsets, and colors from actual bbox math. It is the opposite of hand-wavy: conflicts with the W25 spec are flagged, font weights are explicitly [VERIFY] because outlined text cannot prove weight. Its measurable claims check against the repo's real token/SVG artifacts.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "SVG XML in `design/figma/` is ground truth (parsed: every rect, path bbox, fill/stroke...)" | :7-9 | REAL | method matches observed SVG structure (e.g. sidebar.svg 482 paths with precise coords) |
| 2 | "artboard stroke-scaling factors (0.825/1.125/1.28571, themes/tokens.json header)" | :11-12 | REAL | themes/tokens.json:2 $description contains exactly "0.825/1.125/1.28571" |
| 3 | "volume gradient `#95D318 #B2A918 #FF3737`" | :26 | REAL | rg in design/figma/volume.svg -> 24 hits for 95D318/FF3737 |
| 4 | "`#969696` (sidebar rows, compact-dropdown labels) is NOT yet in the token set" | :25-26 | REAL | themes/tokens.json: 969696 appears ONLY inside a $description (line 112, "measures #969696 ... spec value pinned" — token $value is #9A9A9A); as a token value the claim holds |
| 5 | "Weights are [VERIFY] (outlined shapes can't prove 400 vs 500...)" | :14-15 | REAL | honest limitation, correctly reasoned |
| 6 | "12 option rows + 1 separator (height = 4 + 48×12 + 1 + 4 = 585)" | :16-20 | REAL-UNPROVEN | arithmetic checks (585 = 4+576+1+4 ✓); frame-level measurement not re-derived by me |

## 3. Contradictions
None surviving verification (the 969696 nuance resolves in the doc's favor — see ledger row 4).

## 4. Fabrication / inflation indicators
None. The doc consistently downgrades certainty ([VERIFY] on weights, conflicts flagged, "SVG wins" precedence stated).
