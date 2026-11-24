# CLAIM AUDIT: design/figma/Onboarding-8.svg
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 2 |
| REAL / REAL-UNPROVEN / FALSE | 2 / 0 / 0 |

## 1. Blunt verdict
Valid Figma export (1090×759, XML parses) — the violet variant of the
onboarding accent-picker family: `#8E42FF` ×10 (selected swatch) with the
other accents drawn once each, window `rx="14"` + `#0E0E0E` as the family
grammar demands. Consistent with doc 08 §1.2's "Onboarding-3…10 iterate the
same screen with each swatch selected".

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid SVG design asset | :1 | REAL | ElementTree parse OK; root 1090×759; `file` -> SVG ASCII. |
| 2 | doc 08 §1.2: onboarding variants iterate accent selection | docs/08:56 | REAL | `rg -o '#8E42FF\|#E15032\|…'` -> #8E42FF ×10 (selected), #E15032/#C49E23/#19A234/#18AAD3/#185DD3/#F052B3 ×1 each; `rx="14"` ×1 + `fill="#0E0E0E"` ×1 — family grammar intact. |

## 3. Contradictions / Fabrication
None.
