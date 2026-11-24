# CLAIM AUDIT: design/figma-renders/Player_-_customize-8.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R4) |
| Claims extracted | 2 (integrity + fidelity-evidence use) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine Figma render: `file` -> "PNG image data, 900 x 506, 8-bit/color RGB,
non-interlaced"; python3 struct confirms 900x506, color-type 2 (RGB), 45,444 bytes.
Last of the Player customize family; SVG source pair `design/figma/Player - customize-8.svg`
exists in the tree.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Render of the /player customize state 8 (panel closed, dock open, media playing) | whole file | REAL | Consumers: `rg -ln 'Player_-_customize-8' docs/` -> w26.l02.md:24 (MATCH, measurement), w26.g09.md:26 (PARTIAL, dock-width cross-lane defect). |
| 2 | Fidelity-evidence usage (oracle for state8 diffing) | whole file | REAL | w26.g09.md row 8 cites it against `state8-media.png`/`state8-final.png`/`state8-1024.png` shots; w26.l02.md row 13 grades a 1440/1024 capture against it. |

## 3. Contradictions / 4. Fabrication indicators
None. No doc cites this file as evidence of shipped behavior beyond design-oracle
use; both consumers grade the impl AGAINST it (PARTIAL/MATCH), so it functions
as an oracle, not as a proof of implementation.
