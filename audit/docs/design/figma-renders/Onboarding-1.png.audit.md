# DATA AUDIT: design/figma-renders/Onboarding-1.png
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A02) |
| Type / dims | PNG RGBA 900×627 (file(1) + PNG IHDR parsed) |
| SVG counterpart | design/figma/Onboarding-1.svg (1090×759) — 0.826 scale |
| Matrix evidence citations | NONE (rg docs/feature-matrix.toml -> 0 hits) |

## Blunt verdict
Real binary render of the Onboarding appearance step. It is referenced as a
*frame reference* by w26 wave docs (e01.md:10 — "Onboarding-1.png (appearance,
orange selected)" — e01's mapping claim, not this file's), never as a
feature-matrix evidence string. No fabrication exposure.

## Checks run
- `file` -> PNG image data, 900 x 627, 8-bit/color RGBA, non-interlaced (exit 0)
- IHDR parse -> (900, 627); counterpart SVG header 1090×759 (both same
  900/1090 = 627/759 ≈ 0.826 scale family — consistent render set).
- `rg 'Onboarding-1.png' --glob '!audit'` -> 5 wave-26 doc references
  (i01/l01/e01/g01/h01), all as frame references; feature-matrix.toml: 0.

Findings: none. Short data audit per template 12.2.
