# DATA AUDIT: design/figma-renders/thumbnail.png (credit: W19-A04, recovered by W19-R5)
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R5) |
| Date | 2026-11-24 |
| Type / size | `file` -> "PNG image data, 900 x 625, 8-bit/color RGBA, non-interlaced"; 27,424 bytes |
| Dimensions | 900×625 (render canvas; sibling design/figma/thumbnail.svg viewBox 272×189 — same 1.44 aspect) |
| Consumers | 8 w26 docs (CONTRACT.md, logs w26.f06/w26.b12, findings w26.g03/w26.g18/w26.e09, ratings w26.h03/w26.h16) + product code: ui/src/screens/playlist/playlist.css:552-566 cites it as the media-thumbnail frame oracle |
| Cited as evidence by matrix/doc rows? | w26.b12.md:16 "thumbnail.png / thumbnail.svg — the card grammar: r14 #191919 panel + 1px #404040 hairline" — the grammar claim re-verified independently in audit/docs/docs/waves/w26/logs/w26.b12.md.audit.md (discovery.css comment :7) |
| Reachable from shipped entry point? | Design asset only; not executed code |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
REAL design asset, intact PNG, actively consumed as the card-grammar oracle by both the w26
program and shipped UI CSS. No findings.
