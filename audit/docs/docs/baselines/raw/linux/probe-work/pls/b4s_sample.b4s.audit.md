# CLAIM AUDIT: docs/baselines/raw/linux/probe-work/pls/b4s_sample.b4s (XML playlist fixture)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 |
| REAL / ... / FALSE | 1 REAL |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file` -> XML 1.0 document, ASCII. Well-formed WinampXML playlist with one entry pointing
at the probe tone (absolute scratch path recorded at generation time — the raw-capture
honesty is that the path was real when generated). Cited by
docs/inventory/corpus-plan.md. No claims beyond being a probe fixture.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid B4S (Winamp XML) probe playlist | whole file | REAL | file(1); XML parses visually (WinampXML/playlist/entry structure). |

## 3-4. Contradictions / fabrication indicators
None; none.
