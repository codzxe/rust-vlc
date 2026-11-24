# DATA AUDIT: design/figma-renders/sidebar.png (credit: W19-A04, recovered by W19-R5)
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R5) |
| Date | 2026-11-24 |
| Type / size | `file` -> "PNG image data, 900 x 206, 8-bit/color RGBA, non-interlaced"; 32,763 bytes |
| Dimensions | 900×206 (render canvas; sibling design/figma/sidebar.svg viewBox 4492×1028 — same 4.37 aspect: the wide sheet carrying the sidebar grammar rows) |
| Consumers | 4 w26 doc citations (CONTRACT.md, logs w26.a07/w26.b12, findings w26.g03) + product CSS: ui/src/screens/discovery/discovery.css:8 cites "sidebar.png — the quiet 'N items' meta line" grammar; :62 the "15 items" count chip |
| Cited as evidence by matrix/doc rows? | w26.b12 lane log uses it as the Settings-style section-header grammar oracle (verified in this batch's w26.b12 audit) |
| Reachable from shipped entry point? | Design asset only; not executed code |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
REAL design asset, intact PNG; the sidebar/count-chip grammar it pins is consumed by shipped
discovery CSS. No findings.
