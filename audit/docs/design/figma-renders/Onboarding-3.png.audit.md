# DATA AUDIT: design/figma-renders/Onboarding-3.png
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A06) |
| Type / dims | PNG RGBA 900×627 (file(1) verified; PIL re-read) |
| SVG counterpart | design/figma/Onboarding-3.svg (1090×759) — render is 0.826 scale of SVG artboard |
| Matrix evidence citations | NONE (rg 'Onboarding-3.png' repo -> only docs/waves/w26/logs/w26.a06.md:123) |

## Blunt verdict
Real binary render, correct type, no evidence burden. w26.a06.md:123 uses it as
a measurement reference ("big accent-picker rings (100×100 r12 in 120×120" —
w26.a06's own claims, not this PNG's). Nothing here can be a fabricated-evidence
vector: no feature-matrix row cites it.

## Checks run
- `file` -> "PNG image data, 900 x 627, 8-bit/color RGBA, non-interlaced" (exit 0)
- PIL open -> (900, 627) RGBA, header parses; byte size = 71 lines-of-loc claim
  in prompt is irrelevant for binaries (LOC field is a listing artifact).
- `rg 'Onboarding-3.png' --glob '!audit'` -> exactly 1 doc citation (w26.a06.md).

Findings: none. Short data audit per template 12.2 (binary asset).
