# CLAIM AUDIT: design/figma/Audio - Equalizer.svg
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 3 |
| REAL / FALSE | 3 REAL / 0 |

## 1. Blunt verdict
Valid 400×1623 component-level EQ export (XML parses, 150 elements), the
A1-exact comparator cited by w26.c1.md:89 and the eq-rows GAPS baseline of
w26.a04.md:74. Its width is actually 400 as claimed by the tool-panels
family, though w26.j09.md:30 correctly notes sibling panels measure 399/405
(the family prose "every tool frame is 400" overstates). Family grammar
present: rx 4/5/6 + #090909/#191919 fills (23 hits).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid SVG design asset | :1 | REAL | ElementTree parse OK; root 400×1623; `file` -> SVG ASCII. |
| 2 | doc 08:167 — cited in the audio-tools family row | docs/08:167 | REAL | File present in the family listing (Audio - Equalizer.svg, EQ.svg, EQ presets.svg, …). |
| 3 | w26.c1.md:89 — "eq-card vs `Audio_-_Equalizer.png` (the A1-exact comparator)" | docs/waves/w26/logs/w26.c1.md:89 | REAL | The PNG render (Audio_-_Equalizer.png) exists in design/figma-renders/ (directory listing); this SVG is its source. |

## 3. Contradictions / Fabrication
None in-file. (Context note: w26.j09.md:30 measures this file's width as
399 — root attribute says 400; ±1px frame-noise class, not a defect.)
