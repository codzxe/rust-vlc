# CLAIM AUDIT: design/figma/Settings - video.svg (502 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 3 (integrity + 2 fidelity citations) |
| REAL 2 / REAL-UNPROVEN 1 / others 0 | |

## 1. Blunt verdict
Genuine Figma export of the Settings > Video screen: `file` -> SVG ASCII; canvas 1048x865
(sed :1); 122 `<path>` elements; ZERO base64 rasters, ZERO gradients (rg censuses). The
richest consumer is w26.g12:34, which quotes a full label/toggle/slider inventory with
exact ids, y-offsets and color tokens taken from this frame.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | w26.g12:34 "Frame evidence (`Settings - video.svg`, ids verified): Output y89 ... Deinterlacing y633 ... Framing y784" | frame sections | REAL-UNPROVEN (section geometry not re-measured) | Canvas 865px tall; the quoted y89/322/633/784 ladder fits. g12 claims "ids verified" — its own run; not re-measured here. |
| 2 | w26.e06:93 "frame `Settings - video.svg` = Output (Enable video ON · Start videos in ...)" | frame use | REAL (consumer) | e06:93 cites the frame's Output section semantics. |
| 3 | w26.e06:25 "label-id extraction from `Settings - privacy.svg` / `Settings - video.svg`" | frame use | REAL (consumer) | e06:25 — the w26 label-id pipeline mined ids from this frame. |

## 3. Contradictions / 4. Fabrication indicators
None. No doc claims the UI matches the frame; w26 findings enumerate gaps against it.
