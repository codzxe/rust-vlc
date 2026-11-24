# CLAIM AUDIT: design/figma-renders/Settings_-_interface.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 2 (integrity + fidelity-evidence use) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine render: `file` -> "PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced";
PNG signature verified (python3 struct: 900x743, color-type 6). Companion of the Settings
frame family; used as the visual oracle by the w26 loop.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Render of the Settings>Interface screen (family member) | whole file | REAL | Consumers: w26.a06.md, w26.g10.md, w26.e05.md, w26.h08.md, tools-settings.md (rg -ln "Settings_-_interface" docs/). |
| 2 | Fidelity-evidence usage (oracle for UI diffing) | whole file | REAL | w26.h08.md + w26.g10.md cite it in their verification lanes (rg hit lines). |

## 3. Contradictions / 4. Fabrication indicators
None.
