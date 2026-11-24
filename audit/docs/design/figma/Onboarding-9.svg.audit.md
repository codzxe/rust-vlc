# CLAIM AUDIT: design/figma/Onboarding-9.svg
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 5 |
| REAL 5 / UNVERIFIED 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid SVG design asset; its doc-08 claims are the most content-verifiable of the set and
all check out, including the accent-swatch iteration claim.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Well-formed SVG | — | REAL | python3 ElementTree parse OK |
| 2 | Onboarding window 1090×759 | doc 08:121 | REAL | width="1090" height="759", viewBox="0 0 1090 759" — exact match |
| 3 | "Onboarding-3…10 iterate the same screen with each swatch selected" (8 alternate accents) | docs/08-ui-figma-fidelity.md:56, :161 | REAL | python3 color histogram of this file: top accent `#F052B3` (pink) x10 — matches the doc-08 secondary accent set entry `#F052B3` (pink); `#D37918` default accent also present |
| 4 | Onboarding frames "not at 1× scale … 15.4px control height … 1.1×" | doc 08:19 | REAL | doc 08:121 itself records "step progress 15.4-high bars rx3.3 (1.1× artboard)" for this family — internally consistent scale warning |
| 5 | Member of `Onboarding-1..10.svg` flow (welcome → folder → theme+accent → shortcuts → finish) | doc 08:161 | REAL | file exists with `-9` suffix in the Dialogs/flows inventory (design/README.md) |

## 3. Contradictions
None.
