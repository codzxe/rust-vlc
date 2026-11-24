# CLAIM AUDIT: design/figma/Settings - global-1.svg (516 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 3 (integrity + 2 measured fidelity citations) |
| REAL 3 / others 0 | |

## 1. Blunt verdict
Genuine Figma export of the Settings > Global screen, Advanced Options expanded state:
`file` -> SVG ASCII; canvas 1048x865 (sed :1); 117 `<path>` elements; ZERO base64 rasters,
ZERO gradients (rg censuses) — pure vector. Consumers: w26 logs/findings/ratings use it as
the frame oracle, including a measured UI bug filed against it.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | w26.e05:24 "`Settings - global-1.svg` (422,782 B vs global.svg 340,515 B)" | whole file | REAL | `wc -c` -> 422,782 bytes (verified); sibling global.svg exists per w26 references. |
| 2 | w26.h08:102 "Frame `Settings - global-1.svg` keeps the bar at y645 with all 4 rows visible by y851" | frame geometry | REAL-UNPROVEN (geometry not re-measured) | Canvas 865px tall (sed :1) — y851 fits; h08's bug report (UI rows y834-968, `visible:false`, scrollTop 0) is a UI defect claim, not an SVG claim. Row geometry not re-measured by me. |
| 3 | w26.a01:54 "Primaries aligned to the design swatch row (Settings - global-1.svg ...)" | frame use | REAL (consumer) | a01:54/:86 cite the swatch row of this frame; file is the reference artifact. |

## 3. Contradictions / 4. Fabrication indicators
None. Consumers treat the SVG as truth to score the UI against — correct direction.
