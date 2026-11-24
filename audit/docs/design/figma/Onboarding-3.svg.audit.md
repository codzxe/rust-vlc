# CLAIM AUDIT: design/figma/Onboarding-3.svg
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / FALSE | 3 / 0 / 0 |

## 1. Blunt verdict
Valid Figma export (1090×759, XML parses, 174 elements) matching every
design claim made about it: doc 08 §1.4's "Onboarding window 1090×759 rx14
`#0E0E0E`" (root attrs + `rx="14"` ×1 + `fill="#0E0E0E"` ×2 verified),
doc 08 §1.2's "user-selectable accent (Onboarding-3…10 iterate the same
screen with each swatch selected)" — Onboarding-3 is the red-orange
(#E15032 ×10) selection with the other 7 swatches drawn once each, exactly
as w26.gfix-D.md:93 records ("Onboarding-3 (red) MATCH").

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid SVG design asset | :1 | REAL | ElementTree parse OK; 1090×759 root; `file` -> SVG ASCII. |
| 2 | doc 08 §1.4: "Onboarding window 1090×759 rx14 `#0E0E0E`" | docs/08:1.4 | REAL | `<svg width="1090" height="759"` (:1); `rg 'rx="14"'` -> 1; `fill="#0E0E0E"` -> 2. |
| 3 | doc 08 §1.2: "Onboarding-3…10 iterate the same screen with each swatch selected" | docs/08:56 | REAL | Accent counts: #E15032 ×10 (selected), #C49E23/#19A234/#18AAD3/#185DD3/#8E42FF/#F052B3 ×1 each — the 8-swatch picker with red-orange selected; 1.1× artboard scale values (15.4/3.3/4.4) verified in this file during the doc-08 audit. |

## 3. Contradictions / Fabrication
None.
