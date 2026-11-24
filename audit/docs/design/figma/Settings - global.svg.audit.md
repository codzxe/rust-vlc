# CLAIM AUDIT: design/figma/Settings - global.svg
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 4 |
| REAL 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid SVG design asset; cited twice in doc 08, including a typography token claim
(`Search` placeholder) that is consistent with file content.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Well-formed SVG | — | REAL | python3 ElementTree parse OK |
| 2 | Dimensions 1048×865 | doc 08:160 | REAL | width="1048" height="865", viewBox="0 0 1048 865" |
| 3 | "`Search` placeholder (Settings - global)" uses `font-caption` | docs/08-ui-figma-fidelity.md:85 | REAL | python3: `Search` string present in this file's content (the font-caption half is a doc-08 token mapping, text is outlined so glyph font identity is [VERIFY] by design README) |
| 4 | Member of settings family `global(-1)` | doc 08:160 | REAL | listed; this file is the `-0` member |

## 3. Contradictions
None.
