# DATA AUDIT: design/figma-renders/Dropdown.png (credit: W19-A08, recovered by W19-R5)
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R5) |
| Date | 2026-11-24 |
| Type / size | `file` -> "PNG image data, 900 x 818, 8-bit/color RGBA, non-interlaced"; 34,534 bytes |
| Dimensions | 900×818 (render canvas; sibling design/figma/Dropdown.svg viewBox 406×369 — the closed/open dropdown states) |
| Consumers | 5 w26 doc citations (logs w26.a05/a06, ratings w26.h18, findings w26.l04/w26.e12); the shipped Dropdown component family it frames lives in ui/src/components/menu/ (Dropdown.tsx + menu.css — audited in this batch via w26.h13/j09 ledgers) |
| Cited as evidence by matrix/doc rows? | No feature-matrix row; w26 lane logs use it as the dropdown grammar oracle |
| Reachable from shipped entry point? | Design asset only; not executed code |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
REAL design asset, intact PNG; the dropdown-frame oracle for the menu-lane family whose
shipped component was independently audited this batch (trigger ink, list semantics,
scroll fix). No findings.
