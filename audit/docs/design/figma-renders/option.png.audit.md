# DATA AUDIT: design/figma-renders/option.png
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A07) |
| Type / dims | PNG RGBA 900×873 (file(1) + IHDR parse) |
| SVG counterpart | design/figma/option.svg (301×292) — render is a wider cropped composition, not a 1:1 scale of the small SVG artboard |
| Matrix evidence citations | NONE (rg docs/feature-matrix.toml -> 0) |

## Blunt verdict
Real binary render, menu-lane asset. Declared in the wave contract's
components lane (CONTRACT.md:57 "Menus | components | ... `option.png` ..."),
used as a frame reference by shortcuts.css:219 ("frame: option.png — the
selected row keeps its quiet #8F8F8F label") and by 7 wave-26 docs. No
feature-matrix evidence row cites it; no fabrication exposure.

## Checks run
- `file` -> PNG image data, 900 x 873, 8-bit/color RGBA, non-interlaced (exit 0)
- IHDR -> (900, 873); counterpart option.svg header 301×292.
- `rg 'option\.png' --glob '!audit'` -> 8 files (CONTRACT, gfix-C, a18, l04,
  g20, e12, h18, shortcuts.css); feature-matrix.toml: 0 hits.

Findings: none. Short data audit per template 12.2.
