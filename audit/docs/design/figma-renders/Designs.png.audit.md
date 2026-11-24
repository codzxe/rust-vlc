# DATA AUDIT: design/figma-renders/Designs.png
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A04) |
| Type / dims | PNG RGBA 900×288 (file(1) + IHDR parse) |
| SVG counterpart | design/figma/Designs.svg (2645×847 — wide system sheet, ~0.34 scale render) |
| Matrix evidence citations | NONE |

## Blunt verdict
Real binary render of the wide "Designs" system sheet. The only reference is
docs/waves/w26/CONTRACT.md:60 — "System sheets | reference only | Components.png,
Designs.png" — i.e. declared *reference-only*, excluded from the frame-mapping
contract. No feature-matrix row cites it. No evidence burden, no exposure.

## Checks run
- `file` -> PNG image data, 900 x 288, 8-bit/color RGBA, non-interlaced (exit 0)
- IHDR -> (900, 288); SVG header width=2645 height=847 (aspect 3.125 matches
  render aspect 900/288 = 3.125 exactly — same artboard, scaled).
- `rg 'Designs.png' --glob '!audit'` -> 1 hit (CONTRACT.md:60, reference-only).

Findings: none. Short data audit per template 12.2.
