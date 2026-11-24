# CLAIM AUDIT: design/figma-renders/Settings_-_extensions.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R4) |
| Claims extracted | 2 (integrity + fidelity-evidence use) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine render: `file` -> "PNG image data, 900 x 743, 8-bit/color RGBA,
non-interlaced"; python3 struct confirms 900x743, color-type 6 (RGBA), 46,254
bytes. SVG source twin `design/figma/Settings - extensions.svg` present in tree.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Render of the Settings>Extensions screen (extension rows with enable toggles) | whole file | REAL | `rg -ln 'Settings_-_extensions' docs/` -> w26.b08.md:9 "extension rows with enable toggles (form API ...)", w26.h12.md:40 "## 3. /settings/extensions vs `Settings_-_extensions.png`". |
| 2 | Fidelity-evidence usage (oracle for the extensions settings lane) | whole file | REAL | w26.h12.md grades /settings/extensions against this render; w26.b08.md builds the extensions screen per this frame. |

## 3. Contradictions / 4. Fabrication indicators
None. Same 900x743 dimensions as the rest of the Settings family renders
(interface 900x743 verified in sibling audits) — family-consistent export.
